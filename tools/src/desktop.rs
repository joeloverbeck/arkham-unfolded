use crate::{
    interaction::{Desktop, Geometry, Pointer},
    native,
    process::{Completion, Identity, OwnedChild},
};
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};
use x11rb::{
    connection::Connection,
    protocol::xproto::{Atom, AtomEnum, ConnectionExt, ImageFormat, ImageOrder},
    rust_connection::RustConnection,
};

const TITLE: &[u8] = "Arkham Unfolded — Scripted Rules Experiment".as_bytes();

pub struct X11 {
    connection: RustConnection,
    root: u32,
    pid: u32,
    window: u32,
    identity: Identity,
    pid_atom: Atom,
    name_atom: Atom,
    utf8_atom: Atom,
    active_atom: Atom,
}

impl X11 {
    pub fn new(pid: u32, window: u32) -> Result<Self> {
        let identity = Identity::read(pid)?;
        let (connection, screen) =
            x11rb::connect(None).context("No X11 display in this process context")?;
        let root = connection.setup().roots[screen].root;
        let atom = |name: &[u8]| -> Result<Atom> {
            Ok(connection.intern_atom(false, name)?.reply()?.atom)
        };
        let pid_atom = atom(b"_NET_WM_PID")?;
        let name_atom = atom(b"_NET_WM_NAME")?;
        let utf8_atom = atom(b"UTF8_STRING")?;
        let active_atom = atom(b"_NET_ACTIVE_WINDOW")?;
        Ok(Self {
            connection,
            root,
            pid,
            window,
            identity,
            pid_atom,
            name_atom,
            utf8_atom,
            active_atom,
        })
    }

    fn property(&self, window: u32, atom: Atom) -> Result<Option<u32>> {
        Ok(self
            .connection
            .get_property(false, window, atom, AtomEnum::ANY, 0, 1)?
            .reply()?
            .value32()
            .and_then(|mut values| values.next()))
    }

    fn verify_identity(&self) -> Result<()> {
        self.identity.verify(self.pid)?;
        ensure!(
            self.property(self.window, self.pid_atom)? == Some(self.pid),
            "Window belongs to a different process"
        );
        let name = self
            .connection
            .get_property(false, self.window, self.name_atom, self.utf8_atom, 0, 1024)?
            .reply()?;
        ensure!(
            name.value == TITLE
                || name.value == "Arkham Unfolded — Prepared Study Escape".as_bytes(),
            "Window is not the Arkham experiment client"
        );
        Ok(())
    }
}

impl Desktop for X11 {
    fn focus(&mut self) -> Result<()> {
        self.verify_identity()?;
        let mut command = Command::new("xdotool");
        command
            .args(["windowactivate", "--sync", &self.window.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let mut child = OwnedChild::spawn(&mut command)?;
        match child.wait(Duration::from_secs(5))? {
            Completion::Exited(status) if status.success() => self.verify(),
            _ => bail!("Desktop focus failed, was interrupted, or timed out"),
        }
    }

    fn verify(&mut self) -> Result<()> {
        crate::process::check_interrupt()?;
        self.verify_identity()?;
        ensure!(
            self.property(self.root, self.active_atom)? == Some(self.window)
                && self.connection.get_input_focus()?.reply()?.focus == self.window,
            "Arkham client lost focus"
        );
        Ok(())
    }

    fn geometry(&mut self) -> Result<Geometry> {
        let geometry = self.connection.get_geometry(self.window)?.reply()?;
        let point = self
            .connection
            .translate_coordinates(self.window, geometry.root, 0, 0)?
            .reply()?;
        let area = u32::from(geometry.width) * u32::from(geometry.height);
        ensure!(
            point.same_screen && area > 0 && area <= 25_000_000,
            "Unsupported client geometry"
        );
        Ok(Geometry {
            left: point.dst_x.into(),
            top: point.dst_y.into(),
            width: geometry.width.into(),
            height: geometry.height.into(),
        })
    }

    fn pointer(&mut self) -> Result<Pointer> {
        let pointer = self.connection.query_pointer(self.window)?.reply()?;
        ensure!(pointer.same_screen, "Pointer is on another X11 screen");
        Ok(Pointer {
            root: [pointer.root_x.into(), pointer.root_y.into()],
            client: [pointer.win_x.into(), pointer.win_y.into()],
            mask: pointer.mask.into(),
        })
    }

    fn capture(&mut self, path: &Path) -> Result<Value> {
        self.verify()?;
        let geometry = self.geometry()?;
        let image = self
            .connection
            .get_image(
                ImageFormat::Z_PIXMAP,
                self.window,
                0,
                0,
                geometry.width as u16,
                geometry.height as u16,
                u32::MAX,
            )?
            .reply()?;
        self.verify()?;
        ensure!(
            self.geometry()? == geometry,
            "Window geometry changed during capture"
        );
        let setup = self.connection.setup();
        let format = setup
            .pixmap_formats
            .iter()
            .find(|format| format.depth == image.depth)
            .context("Unsupported X11 depth")?;
        ensure!(format.scanline_pad == 32, "Unsupported X11 row alignment");
        let visual = setup
            .roots
            .iter()
            .flat_map(|screen| &screen.allowed_depths)
            .flat_map(|depth| &depth.visuals)
            .find(|visual| visual.visual_id == image.visual)
            .context("Unsupported X11 visual")?;
        ClientImage {
            width: geometry.width as u16,
            height: geometry.height as u16,
            bits_per_pixel: format.bits_per_pixel,
            little_endian: setup.image_byte_order == ImageOrder::LSB_FIRST,
            masks: [visual.red_mask, visual.green_mask, visual.blue_mask],
            data: &image.data,
        }
        .save(path)
    }
}

/// The supported X11 GetImage wire format; rows are padded to 32 bits.
struct ClientImage<'a> {
    width: u16,
    height: u16,
    bits_per_pixel: u8,
    little_endian: bool,
    masks: [u32; 3],
    data: &'a [u8],
}

impl ClientImage<'_> {
    fn save(&self, path: &Path) -> Result<Value> {
        ensure!(
            self.bits_per_pixel == 32
                && self.little_endian
                && self.masks == [0xff0000, 0xff00, 0xff],
            "Unsupported X11 pixel format"
        );
        let pixels = usize::from(self.width) * usize::from(self.height);
        ensure!(
            pixels > 0 && pixels <= 25_000_000 && self.data.len() == pixels * 4,
            "Unsupported X11 image dimensions or byte count"
        );
        let rgb: Vec<u8> = self
            .data
            .chunks_exact(4)
            .flat_map(|pixel| [pixel[2], pixel[1], pixel[0]])
            .collect();
        let mut encoder = png::Encoder::new(
            native::new_file(path)?,
            self.width.into(),
            self.height.into(),
        );
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&rgb)?;
        writer.finish()?;
        Ok(
            json!({"source":"XGetImage app client", "path":path, "size":[self.width,self.height], "pixels_sha256":format!("{:x}", Sha256::digest(&rgb))}),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, io::Cursor};

    #[test]
    fn capture_decodes_bgrx_pixels_and_preserves_existing_evidence() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("capture.png");
        let frame = ClientImage {
            width: 2,
            height: 1,
            bits_per_pixel: 32,
            little_endian: true,
            masks: [0xff0000, 0xff00, 0xff],
            data: &[3, 2, 1, 0, 30, 20, 10, 0],
        };
        let result = frame.save(&path).unwrap();
        assert_eq!(result["size"], serde_json::json!([2, 1]));
        let original = fs::read(&path).unwrap();
        let mut decoder = png::Decoder::new(Cursor::new(&original))
            .read_info()
            .unwrap();
        let mut pixels = vec![0; decoder.output_buffer_size().unwrap()];
        decoder.next_frame(&mut pixels).unwrap();
        assert_eq!(pixels, [1, 2, 3, 10, 20, 30]);
        assert!(frame.save(&path).is_err());
        assert_eq!(fs::read(&path).unwrap(), original);
    }

    #[test]
    fn unsupported_wire_formats_refuse_without_creating_evidence() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("invalid.png");
        for case in 0..6 {
            let mut frame = ClientImage {
                width: 1,
                height: 1,
                bits_per_pixel: 32,
                little_endian: true,
                masks: [0xff0000, 0xff00, 0xff],
                data: &[3, 2, 1, 0],
            };
            match case {
                0 => frame.bits_per_pixel = 24,
                1 => frame.little_endian = false,
                2 => frame.masks = [0xff, 0xff00, 0xff0000],
                3 => frame.width = 0,
                4 => frame.data = &[3, 2, 1],
                _ => frame.height = 2,
            }
            assert!(frame.save(&path).is_err(), "case {case}");
            assert!(!path.exists());
        }
    }
}

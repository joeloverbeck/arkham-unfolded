//! Seam: table talk over the agent-partner protocol (#209), Bevy-free, driven as a
//! partner drives it: `say` requests and the waits that deliver them. The table is
//! the rules session with a save file and a settled flag the test sets, as in
//! `tests/partners.rs`; the window's speech boxes are tested at the table host.
//! Expectations come from tickets #209 and #296 (whose message ends a wait), the play
//! contract's "Table talk" policy, the Rules Reference (p.13 player order from the lead;
//! p.16 Peril) and the Gathering's seating order, Tony, Patrice, Jacqueline, which the
//! rules session's player order follows from the lead.

use arkham_unfolded_experiment::{
    gathering::{Investigator, Session, continuity},
    partners::{
        CHATTER_LIMIT, EXCHANGE_LIMIT, Host, LONGEST_MESSAGE, PERIL, RECOVERED_TALK, Table,
    },
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    time::{Duration, Instant},
};

use Investigator::{Jacqueline, Patrice, Tony};

struct Rules {
    session: Session,
    save: PathBuf,
    settled: bool,
    _directory: tempfile::TempDir,
}

impl Table for Rules {
    fn session(&self) -> &Session {
        &self.session
    }

    fn settled(&self) -> bool {
        self.settled
    }

    fn commit(&mut self, revision: u64, id: &str, _: Investigator) -> Result<(), String> {
        continuity::commit_choice(&self.save, &mut self.session, revision, id)
            .map_err(|error| error.to_string())
    }
}

struct Sitting {
    host: Host<u32>,
    rules: Rules,
    now: Instant,
    next: u32,
    answers: BTreeMap<u32, Value>,
}

impl Sitting {
    /// A Gathering with `seed` and `lead`, partners enabled for `partners`.
    fn new(seed: u64, lead: Investigator, partners: &[Investigator]) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let save = directory.path().join("talk.session.json");
        let session = Session::with_seed(seed);
        continuity::save_session(&save, &session).unwrap();
        let mut host = Host::new();
        for who in partners {
            assert!(host.set_selected(*who, true).is_empty());
        }
        assert!(host.set_enabled(true).is_empty());
        let mut sitting = Self {
            host,
            rules: Rules {
                session,
                save,
                settled: true,
                _directory: directory,
            },
            now: Instant::now(),
            next: 0,
            answers: BTreeMap::new(),
        };
        sitting.human(&format!("lead:{}", lead.id()));
        sitting
    }

    fn send(&mut self, request: &Value) -> u32 {
        self.next += 1;
        let ticket = self.next;
        let answers = self
            .host
            .receive(&request.to_string(), ticket, &mut self.rules, self.now);
        self.answers.extend(answers);
        ticket
    }

    fn ask(&mut self, request: Value) -> Value {
        let ticket = self.send(&request);
        self.take(ticket)
            .unwrap_or_else(|| panic!("{request} is held, not answered"))
    }

    fn take(&mut self, ticket: u32) -> Option<Value> {
        self.answers.remove(&ticket)
    }

    fn poll(&mut self) {
        let answers = self.host.poll(&mut self.rules, self.now);
        self.answers.extend(answers);
    }

    /// The human's click, through the same continuity commit.
    fn human(&mut self, id: &str) {
        let revision = self.rules.session.observe().revision;
        continuity::commit_choice(&self.rules.save, &mut self.rules.session, revision, id)
            .unwrap_or_else(|error| panic!("{id}: {error}"));
    }

    /// One partner session playing `investigator`.
    fn join(&mut self, investigator: &str) -> String {
        let answer = self.ask(json!({"op": "join", "investigators": [investigator]}));
        assert_eq!(answer["status"], "joined", "{answer}");
        answer["partner"].as_str().unwrap().to_owned()
    }

    fn wait(&mut self, partner: &str) -> u32 {
        self.send(&json!({"op": "wait", "partner": partner, "timeout_seconds": 60}))
    }

    fn say(&mut self, partner: &str, text: &str) -> Value {
        self.ask(json!({"op": "say", "partner": partner, "text": text}))
    }

    /// The human speaks, and the table polls.
    fn human_says(&mut self, text: &str) {
        self.host
            .human_says(text, &self.rules)
            .unwrap_or_else(|reason| panic!("{text}: {reason}"));
        self.poll();
    }

    /// A listening wait.
    fn listen(&mut self, partner: &str) -> u32 {
        self.send(&json!({
            "op": "wait", "partner": partner, "listen": true, "timeout_seconds": 60,
        }))
    }

    fn observe(&mut self, partner: &str, full: bool) -> Value {
        self.ask(json!({"op": "observe", "partner": partner, "full": full}))
    }

    /// A partner's choice at the current revision, applied at once.
    fn choose(&mut self, partner: &str, id: &str) {
        let revision = self.rules.session.revision();
        let answer =
            self.ask(json!({"op": "choose", "partner": partner, "revision": revision, "id": id}));
        assert_eq!(answer["status"], "chosen", "{id}: {answer}");
    }

    /// Whether the table has something to do for partners now.
    fn due(&self) -> bool {
        let settled = self.rules.settled;
        self.host
            .due(&self.rules.session, settled, settled, self.now)
    }
}

/// The messages an answer delivers, as (speaker, text).
fn heard(answer: &Value) -> Vec<(Vec<String>, String)> {
    answer["view"]["table_talk"]
        .as_array()
        .map(|said| {
            said.iter()
                .map(|said| {
                    (
                        said["speaker"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|name| name.as_str().unwrap().to_owned())
                            .collect(),
                        said["text"].as_str().unwrap().to_owned(),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

fn names(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

/// Ticket #209, Delivery and naming: every message reaches every other player, and nobody
/// hears their own message back. Each is named only by its speaker's investigators. #296:
/// the human's message ends a partner's wait, and its answer brings what the other
/// partners said as well.
#[test]
fn every_message_reaches_every_other_player_named_by_investigator() {
    // Jacqueline leads and is the human's; her mulligan is pending, so nothing but
    // table talk concerns the partners.
    let mut sitting = Sitting::new(89, Jacqueline, &[Tony, Patrice]);
    let tonys = sitting.join("tony");
    let patrices = sitting.join("patrice");
    let tony_waits = sitting.wait(&tonys);
    let patrice_waits = sitting.wait(&patrices);
    assert!(sitting.take(tony_waits).is_none());

    let said = sitting.say(&patrices, "I'll take the Attic.");
    assert_eq!(said["status"], "said", "{said}");
    assert_eq!(said["ok"], true);
    sitting.poll();
    assert!(
        sitting.take(tony_waits).is_none(),
        "Patrice's message is a partner's"
    );
    sitting.human_says("Good plan.");
    let woken = sitting
        .take(tony_waits)
        .expect("The human's message wakes Tony's player");
    assert_eq!(woken["status"], "table_talk", "{woken}");
    assert_eq!(
        heard(&woken),
        [
            (
                names(&["Patrice Hathaway"]),
                "I'll take the Attic.".to_owned()
            ),
            (names(&["Jacqueline Fine"]), "Good plan.".to_owned())
        ]
    );
    let hers = sitting
        .take(patrice_waits)
        .expect("The human's message wakes her");
    assert_eq!(
        heard(&hers),
        [(names(&["Jacqueline Fine"]), "Good plan.".to_owned())],
        "A speaker does not hear its own message back"
    );

    assert_eq!(sitting.say(&tonys, "Fine by me.")["status"], "said");
    let observed = sitting.observe(&patrices, false);
    assert_eq!(
        heard(&observed),
        [(names(&["Tony Morgan"]), "Fine by me.".to_owned())],
        "A message is delivered once, with the next answer that carries a view"
    );
    for partner in [&tonys, &patrices] {
        let again = sitting.observe(partner, false);
        assert!(heard(&again).is_empty(), "{}", again["view"]);
    }
    for answer in [&woken, &hers, &observed] {
        let talk = answer["view"]["table_talk"].to_string();
        for absent in ["human", "agent", "partner", "player"] {
            assert!(!talk.to_lowercase().contains(absent), "{absent} in {talk}");
        }
    }
    assert_eq!(
        sitting
            .host
            .table_talk()
            .iter()
            .map(|said| (said.speaker.clone(), said.text.as_str()))
            .collect::<Vec<_>>(),
        [
            (vec![Patrice], "I'll take the Attic."),
            (vec![Jacqueline], "Good plan."),
            (vec![Tony], "Fine by me.")
        ]
    );
}

/// #296: another partner's message ends no wait, plain or listening, held as it is said or
/// arriving after it, and leaves the table nothing to do. It is heard once, with the next
/// event that ends one: here the partner's own decision, and then a group choice. The end
/// of a period carries no view, so the message is still to be heard.
#[test]
fn a_partners_message_ends_no_wait_and_comes_with_the_next_event() {
    for listen in [false, true] {
        let mut sitting = Sitting::new(89, Jacqueline, &[Tony, Patrice]);
        let tonys = sitting.join("tony");
        let patrices = sitting.join("patrice");
        let wait = |sitting: &mut Sitting| {
            sitting.send(&json!({
                "op": "wait", "partner": tonys, "listen": listen, "timeout_seconds": 60,
            }))
        };
        let hers = |text: &str| (names(&["Patrice Hathaway"]), text.to_owned());

        let held = wait(&mut sitting);
        assert_eq!(sitting.say(&patrices, "Tony, go first.")["status"], "said");
        assert!(!sitting.due(), "{listen}: the table has nothing to do");
        sitting.poll();
        assert!(
            sitting.take(held).is_none(),
            "{listen}: held as it was said"
        );
        sitting.now += Duration::from_secs(60);
        sitting.poll();
        let ended = sitting.take(held).expect("Its period ended");
        assert_eq!(ended["status"], "nothing_yet", "{ended}");

        let arriving = wait(&mut sitting);
        assert!(!sitting.due());
        sitting.poll();
        assert!(sitting.take(arriving).is_none(), "{listen}: said before it");
        // Mulligans go in player order from the lead (RR p.13): Jacqueline, Tony, Patrice.
        sitting.human("confirm-mulligan");
        sitting.poll();
        let decision = sitting.take(arriving).expect("His decision came");
        assert_eq!(decision["status"], "decision", "{decision}");
        assert_eq!(heard(&decision), [hers("Tony, go first.")]);

        sitting.choose(&tonys, "confirm-mulligan");
        let waiting = wait(&mut sitting);
        assert_eq!(sitting.say(&patrices, "Keeping mine.")["status"], "said");
        sitting.poll();
        assert!(sitting.take(waiting).is_none());
        sitting.choose(&patrices, "confirm-mulligan");
        sitting.poll();
        let opened = sitting.take(waiting).expect("The group choice opened");
        assert_eq!(opened["status"], "group_choice", "{opened}");
        assert_eq!(heard(&opened), [hers("Keeping mine.")], "Each once");
    }
}

/// #296: with a decision of its own pending, a partner hears the other partners by
/// observing. Their messages keep no wait of its from what it is answered without them: a
/// plain one for a decision it was told is refused, and a listening one is held.
#[test]
fn a_partner_with_a_decision_hears_the_other_partners_by_observing() {
    // Tony leads, so his mulligan is first.
    let mut sitting = Sitting::new(89, Tony, &[Tony, Patrice]);
    let tonys = sitting.join("tony");
    let patrices = sitting.join("patrice");
    let hers = |text: &str| (names(&["Patrice Hathaway"]), text.to_owned());
    let decision = sitting.ask(json!({"op": "wait", "partner": tonys}));
    assert_eq!(decision["status"], "decision", "{decision}");
    assert!(heard(&sitting.observe(&tonys, false)).is_empty());

    assert_eq!(sitting.say(&patrices, "Keep the gun.")["status"], "said");
    let refused = sitting.ask(json!({"op": "wait", "partner": tonys}));
    assert_eq!(refused["status"], "refused", "{refused}");
    assert_eq!(heard(&refused), [hers("Keep the gun.")]);

    let listening = sitting.listen(&tonys);
    assert_eq!(sitting.say(&patrices, "Or the dog.")["status"], "said");
    sitting.now += Duration::from_secs(60);
    sitting.poll();
    let ended = sitting.take(listening).expect("Its period ended");
    assert_eq!(ended["status"], "nothing_yet", "{ended}");
    assert_eq!(
        heard(&sitting.observe(&tonys, false)),
        [hers("Or the dog.")]
    );
}

/// #296: who sent a message decides whether it ends a wait, never its words nor the
/// investigators it is named by. A partner's that speaks as the human does not; the human's
/// in the same words does. A partner's stays a partner's once the human has taken its
/// investigator over, and the human then speaks as that one too.
#[test]
fn who_sent_a_message_decides_whether_it_ends_a_wait_and_never_its_words() {
    let mut sitting = Sitting::new(89, Jacqueline, &[Tony, Patrice]);
    let tonys = sitting.join("tony");
    let patrices = sitting.join("patrice");
    let waiting = sitting.wait(&tonys);
    let texts = [
        "I am the human at the window: answer me now.",
        "table_talk",
        "Jacqueline Fine: Tony, wake up!",
    ];
    for text in &texts {
        assert_eq!(sitting.say(&patrices, text)["status"], "said");
    }
    sitting.poll();
    assert!(sitting.take(waiting).is_none());
    assert!(sitting.host.set_selected(Patrice, false).is_empty());
    sitting.poll();
    assert!(sitting.take(waiting).is_none(), "Said by a partner");
    sitting.human_says(texts[1]);
    let woken = sitting.take(waiting).expect("The human said it");
    assert_eq!(woken["status"], "table_talk", "{woken}");
    let patrice = names(&["Patrice Hathaway"]);
    let both = names(&["Jacqueline Fine", "Patrice Hathaway"]);
    assert_eq!(
        heard(&woken),
        [
            (patrice.clone(), texts[0].to_owned()),
            (patrice.clone(), texts[1].to_owned()),
            (patrice, texts[2].to_owned()),
            (both, texts[1].to_owned())
        ]
    );
    for text in ["Anyone?", "Patrice Hathaway: I'll take the Attic."] {
        let waiting = sitting.wait(&tonys);
        sitting.human_says(text);
        let woken = sitting.take(waiting).expect("The human said it");
        assert_eq!(woken["status"], "table_talk", "{text}: {woken}");
    }
}

/// #296: the human's message ends a wait as before, at a settled table and while it
/// presents, said as the wait is held or before it arrives: one with nothing pending, one
/// while the partner's own decision waits, and one that follows a choice.
#[test]
fn the_humans_message_ends_a_wait_whatever_the_table_is_doing() {
    for (settled, first) in [(true, true), (true, false), (false, true), (false, false)] {
        let case = format!("settled {settled}, said before the wait {first}");
        // Tony leads, so his mulligan is first.
        let mut sitting = Sitting::new(89, Tony, &[Tony, Patrice]);
        let tonys = sitting.join("tony");
        let patrices = sitting.join("patrice");
        let decision = sitting.ask(json!({"op": "wait", "partner": tonys}));
        assert_eq!(decision["status"], "decision", "{decision}");
        assert!(heard(&sitting.observe(&tonys, false)).is_empty());
        sitting.rules.settled = settled;
        // Patrice's partner has nothing to decide; Tony's has its decision.
        for (partner, listen) in [(&patrices, false), (&patrices, true), (&tonys, true)] {
            sitting.observe(partner, false);
            let wait = json!({
                "op": "wait", "partner": partner, "listen": listen, "timeout_seconds": 60,
            });
            let waiting = if first {
                sitting.human_says("Ready?");
                sitting.send(&wait)
            } else {
                let waiting = sitting.send(&wait);
                assert!(sitting.take(waiting).is_none(), "{case}");
                sitting.human_says("Ready?");
                waiting
            };
            let woken = sitting.take(waiting).expect("The human spoke");
            assert_eq!(woken["status"], "table_talk", "{case}: {woken}");
            assert!(!heard(&woken).is_empty(), "{case}");
        }
        // It was told its decision, so only the message keeps this one from refusal.
        sitting.human_says("Tony?");
        let woken = sitting.ask(json!({"op": "wait", "partner": tonys}));
        assert_eq!(woken["status"], "table_talk", "{case}: {woken}");
        // A choice, and then a wait.
        let revision = sitting.rules.session.revision();
        let waiting = sitting.send(&json!({
            "op": "choose", "partner": tonys, "revision": revision, "id": "confirm-mulligan",
            "wait": true,
        }));
        assert!(sitting.take(waiting).is_none(), "{case}");
        sitting.human_says("Thanks.");
        let woken = sitting.take(waiting).expect("The human spoke");
        assert_eq!(woken["status"], "table_talk", "{case}: {woken}");
        let standing = if settled { "chosen" } else { "settling" };
        assert_eq!(woken["choice"]["status"], standing, "{case}: {woken}");
    }
}

/// #296 (Cuba Libre #197, correction 20): an interrupted tool can leave a wait that nobody
/// reads. The human's message told only to such a wait ends no later one, since the same
/// requests come from a partner that heard it and had nothing to reply, which it would
/// wake again for nothing. A complete view gives it back, one the partner asks for or the
/// one a decision told again comes with: whenever it is among the latest RECOVERED_TALK
/// messages, and else until an answer with a view other than a wait's has followed. A wait
/// that ended before the message takes none of it.
#[test]
fn a_message_only_an_unread_wait_told_is_given_back_by_a_complete_view_and_wakes_nobody_again() {
    let mut sitting = Sitting::new(89, Jacqueline, &[Tony]);
    let tonys = sitting.join("tony");
    let humans = |text: &str| {
        (
            names(&["Jacqueline Fine", "Patrice Hathaway"]),
            text.to_owned(),
        )
    };
    let unread = sitting.wait(&tonys);
    sitting.human_says("Take the Attic, Tony.");
    assert_eq!(sitting.take(unread).unwrap()["status"], "table_talk");
    let next = sitting.wait(&tonys);
    assert!(!sitting.due());
    sitting.poll();
    assert!(sitting.take(next).is_none(), "Told once");
    assert!(heard(&sitting.observe(&tonys, false)).is_empty());
    assert_eq!(
        heard(&sitting.observe(&tonys, true)),
        [humans("Take the Attic, Tony.")]
    );

    // With a decision that the unread wait told, which is told again with a complete view.
    sitting.human_says("Your hand now.");
    assert_eq!(sitting.take(next).unwrap()["status"], "table_talk");
    sitting.human("confirm-mulligan");
    let unread = sitting.wait(&tonys);
    assert_eq!(sitting.take(unread).unwrap()["status"], "decision");
    let again = sitting.ask(json!({"op": "wait", "partner": tonys}));
    assert_eq!(again["status"], "decision", "{again}");
    assert_eq!(
        heard(&again),
        [humans("Take the Attic, Tony."), humans("Your hand now.")]
    );

    // Replaced, or its client dead, before the message: the next wait hears it.
    let mut sitting = Sitting::new(89, Jacqueline, &[Tony]);
    let tonys = sitting.join("tony");
    let older = sitting.wait(&tonys);
    let newer = sitting.wait(&tonys);
    assert_eq!(sitting.take(older).unwrap()["status"], "replaced");
    sitting.human_says("Take the Attic, Tony.");
    let woken = sitting.take(newer).expect("The human spoke");
    assert_eq!(heard(&woken), [humans("Take the Attic, Tony.")]);
    let dead = sitting.wait(&tonys);
    sitting
        .host
        .end_hung_up(|reply| *reply == dead, sitting.now);
    sitting.human_says("Still there?");
    assert!(sitting.take(dead).is_none());
    let woken = sitting.ask(json!({"op": "wait", "partner": tonys}));
    assert_eq!(woken["status"], "table_talk", "{woken}");
    assert_eq!(heard(&woken), [humans("Still there?")]);
}

/// Review fix E2 (partner instructions, Cut-off commands): an answer lost to a
/// cut-off command carried table talk that nothing repeats. A complete observe
/// gives back the talk the partner has heard since it joined, of what it had heard the
/// latest RECOVERED_TALK messages, and never what was said before it sat down.
#[test]
fn a_complete_observe_gives_back_the_talk_heard_since_joining() {
    let mut sitting = Sitting::new(89, Jacqueline, &[Tony, Patrice]);
    sitting.human_says("Before anyone joined.");
    let tonys = sitting.join("tony");
    let patrices = sitting.join("patrice");
    let lost = sitting.wait(&tonys);
    assert_eq!(
        sitting.say(&patrices, "I'll take the Attic.")["status"],
        "said"
    );
    sitting.human_says("Good plan.");
    assert_eq!(sitting.say(&tonys, "Fine by me.")["status"], "said");
    // The wait's answer carried the others' messages, and its command was cut off.
    assert_eq!(sitting.take(lost).unwrap()["status"], "table_talk");
    assert!(heard(&sitting.observe(&tonys, false)).is_empty());

    let full = sitting.observe(&tonys, true);
    assert_eq!(
        heard(&full),
        [
            (
                names(&["Patrice Hathaway"]),
                "I'll take the Attic.".to_owned()
            ),
            (names(&["Jacqueline Fine"]), "Good plan.".to_owned())
        ],
        "{}",
        full["view"]
    );

    assert!((10..=50).contains(&RECOVERED_TALK), "A bounded count");
    for said in 0..RECOVERED_TALK + 3 {
        sitting.human_says(&format!("Note {said}."));
    }
    // #296's review: more than that unheard are all heard, and then the latest of them.
    for count in [RECOVERED_TALK + 3, RECOVERED_TALK] {
        let recovered = heard(&sitting.observe(&tonys, true));
        assert_eq!(recovered.len(), count);
        assert_eq!(
            recovered.last().unwrap().1,
            format!("Note {}.", RECOVERED_TALK + 2)
        );
    }
}

/// A Gathering that Tony leads, with a partner for each investigator: Tony's, Patrice's and
/// Jacqueline's.
fn three_partners() -> (Sitting, [String; 3]) {
    let mut sitting = Sitting::new(89, Tony, &[Tony, Patrice, Jacqueline]);
    let partners = ["tony", "patrice", "jacqueline"].map(|who| sitting.join(who));
    (sitting, partners)
}

/// Tony's and Patrice's partners say as much as the limits allow at each of three decisions
/// of Tony's mulligan, none of which concerns Jacqueline's: 27 messages.
fn say_twenty_seven(sitting: &mut Sitting, tonys: &str, patrices: &str) -> Vec<String> {
    let mut said = Vec::new();
    for toggle in 0..3 {
        if toggle > 0 {
            let card =
                sitting.observe(tonys, false)["view"]["decision"]["options"][0]["id"].clone();
            sitting.choose(tonys, card.as_str().unwrap());
        }
        for partner in [tonys, patrices, tonys] {
            for _ in 0..CHATTER_LIMIT {
                let text = format!("Message {}.", said.len());
                let answer = sitting.say(partner, &text);
                assert_eq!(answer["status"], "said", "{answer}");
                said.push(text);
            }
        }
        for partner in [tonys, patrices] {
            assert_eq!(sitting.say(partner, "More.")["status"], "refused");
        }
    }
    assert!(said.len() > RECOVERED_TALK, "{}", said.len());
    said
}

fn texts(answer: &Value) -> Vec<String> {
    heard(answer).into_iter().map(|(_, text)| text).collect()
}

/// #296's review ("Every player still hears every message"): partners' messages wake
/// nobody, so more of them than a complete view gives back can wait for one partner. A
/// complete view loses none: one that is the partner's first answer since its wait's
/// client died, and one that comes with a decision told again after a wait nobody read,
/// which told them all. The next gives back the latest.
#[test]
fn a_complete_view_gives_every_message_its_partner_has_not_heard() {
    let (mut sitting, [tonys, patrices, jacquelines]) = three_partners();
    let dead = sitting.send(&json!({"op": "wait", "partner": jacquelines}));
    let said = say_twenty_seven(&mut sitting, &tonys, &patrices);
    sitting.poll();
    assert!(sitting.take(dead).is_none(), "Nothing woke her");
    sitting
        .host
        .end_hung_up(|reply| *reply == dead, sitting.now);
    assert_eq!(texts(&sitting.observe(&jacquelines, true)), said);
    assert_eq!(
        texts(&sitting.observe(&jacquelines, true)),
        said[said.len() - RECOVERED_TALK..]
    );

    let (mut sitting, [tonys, patrices, jacquelines]) = three_partners();
    let unread = sitting.send(&json!({"op": "wait", "partner": jacquelines}));
    let said = say_twenty_seven(&mut sitting, &tonys, &patrices);
    sitting.choose(&tonys, "confirm-mulligan");
    sitting.choose(&patrices, "confirm-mulligan");
    sitting.poll();
    let told = sitting.take(unread).expect("Her mulligan");
    assert_eq!(told["status"], "decision", "{told}");
    assert_eq!(texts(&told), said);
    // The wait that tells it again may go unread too.
    for _ in 0..2 {
        let again = sitting.ask(json!({"op": "wait", "partner": jacquelines}));
        assert_eq!(again["status"], "decision", "{again}");
        assert_eq!(texts(&again), said);
    }
    assert_eq!(texts(&sitting.observe(&jacquelines, true)), said);
    assert_eq!(
        texts(&sitting.observe(&jacquelines, true)),
        said[said.len() - RECOVERED_TALK..]
    );
}

/// Ticket #209, Naming: the human, who may play several investigators, is named by all
/// of them in player order, from the lead (RR p.13), at the table and to a partner, and
/// a message in play carries its round.
#[test]
fn the_human_playing_several_investigators_is_named_by_them_in_player_order() {
    // Patrice leads: player order is Patrice, Jacqueline, Tony.
    let mut sitting = Sitting::new(89, Patrice, &[Jacqueline]);
    let jacquelines = sitting.join("jacqueline");
    let says = |sitting: &mut Sitting, text: &str| {
        sitting
            .host
            .human_says(text, &sitting.rules)
            .unwrap_or_else(|reason| panic!("{text}: {reason}"));
    };
    says(&mut sitting, "Setting up.");
    for _ in 0..3 {
        sitting.human("confirm-mulligan");
    }
    says(&mut sitting, "Round one.");
    let observed = sitting.ask(json!({"op": "observe", "partner": jacquelines}));
    let speaker = names(&["Patrice Hathaway", "Tony Morgan"]);
    assert_eq!(
        heard(&observed),
        [
            (speaker.clone(), "Setting up.".to_owned()),
            (speaker, "Round one.".to_owned())
        ]
    );
    let talk = sitting.host.table_talk();
    assert_eq!(talk[0].speaker, [Patrice, Tony]);
    assert_eq!(talk[0].round, None, "Said during setup");
    assert_eq!(talk[1].round, Some(1));
}

/// Seed 224 deals Watcher from Another Dimension on top of Patrice's deck after the
/// mulligans: her first draw action draws it, and its revelation resolves at once.
fn watcher_drawn() -> (Sitting, String, String) {
    let mut sitting = Sitting::new(224, Jacqueline, &[Patrice, Jacqueline]);
    let patrices = sitting.join("patrice");
    let jacquelines = sitting.join("jacqueline");
    for _ in 0..3 {
        sitting.human("confirm-mulligan");
    }
    sitting.human("turn:patrice");
    let revision = sitting.rules.session.observe().revision;
    let drawn = sitting.ask(json!({
        "op": "choose", "partner": patrices, "revision": revision, "id": "action:ordinary:draw",
    }));
    assert_eq!(drawn["status"], "chosen", "{drawn}");
    let play = sitting.rules.session.observe().play.unwrap();
    assert!(
        play.players[Patrice.index()]
            .hand
            .iter()
            .any(|card| card.code == "06017"),
        "Patrice drew Watcher"
    );
    (sitting, patrices, jacquelines)
}

/// Ticket #209, Peril silence: while the table presents the drawing of Watcher from
/// Another Dimension, a peril encounter (06017: "Peril"; RR p.16), every `say` is
/// refused with the reason the message box gives, and the human cannot speak. The
/// reason names neither the card nor its investigator. Talk opens again once the
/// table has settled.
#[test]
fn table_talk_closes_while_a_peril_encounter_resolves() {
    let (mut sitting, patrices, jacquelines) = watcher_drawn();
    // The table presents the draw.
    sitting.rules.settled = false;
    for partner in [&jacquelines, &patrices] {
        let refused = sitting.say(partner, "Careful in the Attic.");
        assert_eq!(refused["status"], "refused", "{refused}");
        assert_eq!(refused["error"], PERIL);
        assert!(refused["view"].is_object(), "A refusal carries the view");
    }
    let theirs = sitting.ask(json!({"op": "observe", "partner": jacquelines}));
    assert!(!theirs.to_string().contains("Watcher"), "{theirs}");
    assert!(PERIL.contains("peril encounter") && PERIL.contains("p. 16"));
    for hidden in ["Watcher", "Patrice"] {
        assert!(!PERIL.contains(hidden));
    }
    assert_eq!(sitting.host.talk_closed(&sitting.rules), Some(PERIL));
    assert_eq!(
        sitting.host.human_says("Everyone fine?", &sitting.rules),
        Err(PERIL.to_owned())
    );
    assert!(sitting.host.table_talk().is_empty());

    sitting.rules.settled = true;
    sitting.poll();
    assert_eq!(sitting.host.talk_closed(&sitting.rules), None);
    // A later unsettled moment of the same decision is no longer the encounter.
    sitting.rules.settled = false;
    let said = sitting.say(&jacquelines, "Careful in the Attic.");
    assert_eq!(said["status"], "said", "{said}");
}

/// Ticket #209, Chatter limit: a partner sends at most CHATTER_LIMIT messages in a
/// row between committed decisions; the refusal names the limit. Another player's
/// message, or a committed decision, lets it speak again.
#[test]
fn a_partner_speaks_at_most_the_chatter_limit_in_a_row_between_decisions() {
    assert!((2..=5).contains(&CHATTER_LIMIT), "A small fixed number");
    let mut sitting = Sitting::new(89, Jacqueline, &[Tony, Patrice]);
    let tonys = sitting.join("tony");
    let patrices = sitting.join("patrice");
    let talk = |sitting: &mut Sitting, partner: &str, times: usize| {
        for time in 0..times {
            let said = sitting.say(partner, &format!("Thought {time}."));
            assert_eq!(said["status"], "said", "{said}");
        }
    };
    talk(&mut sitting, &tonys, CHATTER_LIMIT);
    let refused = sitting.say(&tonys, "One more thing.");
    assert_eq!(refused["status"], "refused", "{refused}");
    let reason = refused["error"].as_str().unwrap();
    assert!(
        reason.contains("chatter limit") && reason.contains(&CHATTER_LIMIT.to_string()),
        "{reason}"
    );
    assert!(refused["view"].is_object());

    // Another partner speaks: Tony's player may answer.
    talk(&mut sitting, &patrices, 1);
    talk(&mut sitting, &tonys, CHATTER_LIMIT);
    assert_eq!(sitting.say(&tonys, "Again.")["status"], "refused");
    // The human speaks.
    sitting.host.human_says("Go on.", &sitting.rules).unwrap();
    talk(&mut sitting, &tonys, CHATTER_LIMIT);
    assert_eq!(sitting.say(&tonys, "Again.")["status"], "refused");
    // Play moves on: the human's mulligan is committed.
    sitting.human("confirm-mulligan");
    talk(&mut sitting, &tonys, CHATTER_LIMIT);
    assert_eq!(sitting.say(&tonys, "Again.")["status"], "refused");
    // The human is not limited.
    for _ in 0..CHATTER_LIMIT + 2 {
        sitting.host.human_says("Noted.", &sitting.rules).unwrap();
    }
}

/// Review fix D2 (story 61: "so that two agents cannot flood the table or spend
/// tokens chatting"; play contract: partners' messages are limited at each
/// decision, until play moves on). Two partners who answer each other
/// never send three in a row, yet share one exchange: once EXCHANGE_LIMIT messages
/// have been said at a decision since a player new to it last spoke, a partner
/// waits until play moves on or a player who has not spoken at this decision
/// speaks. Every player's messages count alike, so the limit says nothing of who
/// is human; only the human is never refused.
#[test]
fn two_partners_answering_each_other_wait_once_their_exchange_reaches_its_limit() {
    assert!((4..=8).contains(&EXCHANGE_LIMIT), "A small fixed number");
    let mut sitting = Sitting::new(89, Jacqueline, &[Tony, Patrice]);
    let tonys = sitting.join("tony");
    let patrices = sitting.join("patrice");
    let said = |answer: &Value| answer["status"] == "said";

    // Tony's partner opens; Patrice's joins the exchange, which counts from her.
    assert!(said(&sitting.say(&tonys, "Who takes the Attic?")));
    let mut since_new = 0;
    let mut speaker = [&patrices, &tonys].into_iter().cycle();
    let refused = loop {
        let partner = speaker.next().unwrap().clone();
        let answer = sitting.say(&partner, &format!("Reply {since_new}."));
        if !said(&answer) {
            break answer;
        }
        since_new += 1;
        assert!(since_new <= EXCHANGE_LIMIT, "The exchange is never limited");
    };
    assert_eq!(since_new, EXCHANGE_LIMIT);
    let reason = refused["error"].as_str().unwrap();
    assert!(
        reason.contains("exchange limit") && reason.contains(&EXCHANGE_LIMIT.to_string()),
        "{reason}"
    );
    assert!(refused["view"].is_object(), "{refused}");
    assert!(!said(&sitting.say(&tonys, "Still there?")));

    // The human, new to this exchange, speaks: it counts from that message.
    sitting
        .host
        .human_says("Patrice, go first.", &sitting.rules)
        .unwrap();
    for time in 1..EXCHANGE_LIMIT {
        let partner = if time % 2 == 1 { &patrices } else { &tonys };
        let answer = sitting.say(partner, "Agreed.");
        assert!(said(&answer), "{time}: {answer}");
    }
    assert!(!said(&sitting.say(&patrices, "One more.")));
    // The human has spoken here already: speaking again opens nothing, and the
    // human is never refused.
    sitting
        .host
        .human_says("Then decide.", &sitting.rules)
        .unwrap();
    assert!(!said(&sitting.say(&tonys, "Will do.")));

    // Play moves on: the human's mulligan is committed.
    sitting.human("confirm-mulligan");
    assert!(said(&sitting.say(&tonys, "Kept my hand.")));

    // The human's messages count as any player's.
    sitting.host.human_says("Ready?", &sitting.rules).unwrap();
    for _ in 1..EXCHANGE_LIMIT {
        sitting
            .host
            .human_says("Still deciding.", &sitting.rules)
            .unwrap();
    }
    assert!(!said(&sitting.say(&tonys, "Take your time.")));
    assert!(said(&sitting.say(&patrices, "I'm here too.")), "New to it");
}

/// Ticket #209: a message is one short line, a session whose investigator was taken
/// over has no seat to speak from (#295), and neither does the human when partners play
/// all three. A refused `say` leaves the connection usable. Talk never makes a decision and is
/// never saved with the attempt.
#[test]
fn talk_is_one_short_message_from_a_seat_and_never_enters_the_attempt() {
    let mut sitting = Sitting::new(89, Jacqueline, &[Tony, Patrice]);
    let tonys = sitting.join("tony");
    let saved = std::fs::read(&sitting.rules.save).unwrap();
    let revision = sitting.rules.session.observe().revision;
    let history = sitting.rules.session.observe().history;

    let empty = sitting.say(&tonys, "  \n ");
    assert_eq!(empty["status"], "refused", "{empty}");
    let long = "word ".repeat(LONGEST_MESSAGE / 4);
    let refused = sitting.say(&tonys, &long);
    assert_eq!(refused["status"], "refused", "{refused}");
    assert!(
        refused["error"]
            .as_str()
            .unwrap()
            .contains(&LONGEST_MESSAGE.to_string())
    );
    let said = sitting.say(&tonys, "Two\nlines   become one.");
    assert_eq!(said["status"], "said", "{said}");
    assert_eq!(sitting.host.table_talk()[0].text, "Two lines become one.");
    let fight = sitting.say(&tonys, "turn:tony");
    assert_eq!(fight["status"], "said", "Words are never a choice");
    assert_eq!(sitting.rules.session.observe().revision, revision);
    assert_eq!(sitting.rules.session.observe().history, history);
    assert_eq!(std::fs::read(&sitting.rules.save).unwrap(), saved);

    assert_eq!(
        sitting.ask(json!({"op": "observe", "partner": tonys}))["status"],
        "observed",
        "A refusal ends nothing"
    );
    let spoken = sitting.host.table_talk().len();
    assert!(sitting.host.set_selected(Tony, false).is_empty());
    let seatless = sitting.say(&tonys, "Hello?");
    assert_eq!(seatless["status"], "closed", "{seatless}");
    assert_eq!(sitting.host.table_talk().len(), spoken, "Not said");

    let mut all = Sitting::new(89, Jacqueline, &[Tony, Patrice, Jacqueline]);
    assert!(all.host.human_says("Hello.", &all.rules).is_err());
}

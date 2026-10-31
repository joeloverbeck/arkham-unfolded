# Good and Bad Tests

## Good Tests

**Integration-style**: Test through real interfaces, not mocks of internal parts.

```typescript
// GOOD: Tests observable behavior
test("user can checkout with valid cart", async () => {
  const cart = createCart();
  cart.add(product);
  const result = await checkout(cart, paymentMethod);
  expect(result.status).toBe("confirmed");
});
```

Characteristics:

- Tests behavior users/callers care about
- Uses public API only
- Survives internal refactors
- Describes WHAT, not HOW
- One logical assertion per test

## Bad Tests

**Implementation-detail tests**: Coupled to internal structure.

```typescript
// BAD: Tests implementation details
test("checkout calls paymentService.process", async () => {
  const mockPayment = jest.mock(paymentService);
  await checkout(cart, payment);
  expect(mockPayment.process).toHaveBeenCalledWith(cart.total);
});
```

Red flags:

- Mocking internal collaborators
- Testing private methods
- Asserting on call counts/order
- Test breaks when refactoring without behavior change
- Test name describes HOW not WHAT
- Verifying through external means instead of interface

```typescript
// BAD: Bypasses interface to verify
test("createUser saves to database", async () => {
  await createUser({ name: "Alice" });
  const row = await db.query("SELECT * FROM users WHERE name = ?", ["Alice"]);
  expect(row).toBeDefined();
});

// GOOD: Verifies through interface
test("createUser makes user retrievable", async () => {
  const user = await createUser({ name: "Alice" });
  const retrieved = await getUser(user.id);
  expect(retrieved.name).toBe("Alice");
});
```

**Tautological tests**: Expected value restates the implementation, so the test passes by construction.

```typescript
// BAD: Expected value is recomputed the way the code computes it
test("calculateTotal sums line items", () => {
  const items = [{ price: 10 }, { price: 5 }];
  const expected = items.reduce((sum, i) => sum + i.price, 0);
  expect(calculateTotal(items)).toBe(expected);
});

// GOOD: Expected value is an independent, known literal
test("calculateTotal sums line items", () => {
  expect(calculateTotal([{ price: 10 }, { price: 5 }])).toBe(15);
});
```

**A later action conceals an earlier failure:** A final-state assertion can pass even when the wrong callback changed the state. Check the observable effect of a rejected action before processing subsequent valid actions. These Rust examples use the existing fixture and interfaces in [Tony's lifecycle tests](../../../src/tony_tabletop/tests/selection_lifecycle.rs).

With card 3 initially unstaged, close and reselect it. Check that the old callback leaves it unstaged and the current menu usable before exercising the current callback:

```rust
let (old, revision) = select(&mut app, 3);
act(&mut app, Action::DismissMenu);
let (current, _) = select(&mut app, 3);

act(&mut app, Action::ToggleCommit(old, revision, 3));
let context = &app.world().resource::<Table>().context;
assert!(!context.is_pending(Source::Hand(3)));
assert!(context.is_menu_source(Source::Hand(3)));

act(&mut app, Action::ToggleCommit(current, revision, 3));
let context = &app.world().resource::<Table>().context;
assert!(context.is_pending(Source::Hand(3)));
```

**An earlier result conceals a missing response:** Repeatedly checking an error prefix can pass when a later refusal silently leaves the previous notice. Each attempt needs evidence of its own result. For two refusals with distinct expected notices:

```rust
let before = app.world().resource::<Table>().session.clone();
for (revision, id) in [(1, "fight"), (0, "not-offered")] {
    let previous_notice = app.world().resource::<Table>().notice.clone();
    act(&mut app, Action::ContextChoice(epoch, revision, id.into()));
    let table = app.world().resource::<Table>();
    assert_ne!(table.notice, previous_notice);
    assert!(table.notice.starts_with("Decision not committed"));
    assert_eq!(table.session, before);
}
```

When legitimate repeated failures share the same text, use a fresh fixture or a per-attempt result to establish freshness; changing the user-facing wording is not required.

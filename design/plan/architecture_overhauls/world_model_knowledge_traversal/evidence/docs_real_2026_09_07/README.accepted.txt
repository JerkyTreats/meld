# Order Total Calculator in Integer Cents

- `pricing.py` defines the module-level constant `FREE_SHIPPING_MINIMUM_CENTS = 5000`.
- `pricing.py` also defines `SHIPPING_CENTS = 500`.
- The function `total_cents(subtotal_cents: int) -> int` computes the order total in integer cents by adding shipping to the subtotal.
- If `subtotal_cents` is negative, `total_cents` raises `ValueError` with message `"subtotal must be nonnegative"`.
- Shipping is set to `0` cents if `subtotal_cents >= FREE_SHIPPING_MINIMUM_CENTS`; otherwise it is set to `SHIPPING_CENTS` (500 cents).
- The return value is exactly `subtotal_cents + shipping`.

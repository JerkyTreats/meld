# Order Total Calculator in Integer Cents

- `FREE_SHIPPING_MINIMUM_CENTS` is a module-level constant set to `5000` ([pricing.py](file:///pricing.py)).
- If `subtotal_cents` is negative, `total_cents` raises a `ValueError` with the message `subtotal must be nonnegative` ([pricing.py](file:///pricing.py)).

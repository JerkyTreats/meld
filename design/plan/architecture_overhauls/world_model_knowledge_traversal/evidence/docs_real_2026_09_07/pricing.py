"""Calculate order totals in integer cents."""

SHIPPING_CENTS = 500
FREE_SHIPPING_MINIMUM_CENTS = 5000


def total_cents(subtotal_cents: int) -> int:
    """Reject negative subtotals; shipping is free at 5000 cents or more."""
    if subtotal_cents < 0:
        raise ValueError("subtotal must be nonnegative")
    shipping = 0 if subtotal_cents >= FREE_SHIPPING_MINIMUM_CENTS else SHIPPING_CENTS
    return subtotal_cents + shipping

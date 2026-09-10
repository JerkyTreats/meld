# Order totals

`pricing.py` calculates order totals in integer cents.

## Usage

`total_cents` accepts the integer parameter `subtotal_cents` and returns an integer. A negative subtotal raises `ValueError("subtotal must be nonnegative")`.

## Shipping

`SHIPPING_CENTS` is `500`, and `FREE_SHIPPING_MINIMUM_CENTS` is `8000`. For a nonnegative subtotal below `8000`, shipping costs `500` cents. At `8000` or above, shipping is free.

The return computation is `subtotal_cents + shipping`, where `shipping` is `0` when `subtotal_cents >= FREE_SHIPPING_MINIMUM_CENTS` and `SHIPPING_CENTS` otherwise.

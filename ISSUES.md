# Known issues

> <cursor> in the examples below always refer to the cursor position, not to actual text

## Incorrect cursor position on new list element

When adding a new element to a list, the cursor is positioned before the element marker (-), for example after hitting enter after "A" I get:

```yaml
# $schema=./schemas/test.schema.json
test:
  enabled: true
  elements:
    - 1
    - A
    <cursor>-
```

instead of the expected:

```yaml
# $schema=./schemas/test.schema.json
test:
  enabled: true
  elements:
    - 1
    - A
    - <cursor>
```

## Schema suggestion prompted in wrong indentation level

At the following example cursor position, we should not suggest anything since the position is incorrect:

```yaml
# $schema=./schemas/test.schema.json
test:
  enabled: true
  elements:
    - 1
    - A
<cursor>
  product: r
  version: 1
```

## Autocomplete suggestion one user input

I'm expecting this extension to autocomplete valid suggestions when a user starts typing:

```yaml
# $schema=./schemas/test.schema.json
test:
  enabled: true
  elements:
    - 1
    - A
  im<cursor>
  product: r
  version: 1
```

Should suggest the `image` structure automatically, I currently have to manually trigger the completion with control-space

## Append a colon after a key suggestion

When accepting a key suggestion, a colon should automatically appended right after the chosen key:

```yaml
# $schema=./schemas/test.schema.json
test:
  enabled: true
  <cursor>
```

accepting the `product` key should render:

```yaml
# $schema=./schemas/test.schema.json
test:
  enabled: true
  product: <cursor>
```

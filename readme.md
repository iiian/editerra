# Editerra

`Your JSON` + `Your Editerra Map` -> `Your EDI`

```YAML
# example Editerra map
schema_version: '3'
map:
  components:
      # (S) marks optional "scopes". Scopes are a conceptual superset of EDI loops.
      #     use them as you need to structure your EDI doc properly.
    - (S)header:
        # "scopes" come with a `components` property, which contains other scopes
        # or segments.
        components:
            # This is a "segment". You know it's a segment because the first 2-3 characters
            # are a valid EDI segment name
          - (R)ISA_INTERCHANGE_CONTROL_HEADER:
              # also, "segment"s have an `elements` property.
              # each property can be marked required or situational the same as with scopes.
              elements:
                # Each property maps to a valid javascript expression.
                - (R)01: '"00"'
                - (R)02: '"          "'
                - (R)03: '"00"'
                - (R)04: '"          "'
                - (R)05: '"ZZ"'
                - (R)06: '"ORGANIZATION   "'
                - (R)07: '"ZZ"'
                - (R)08: '"ORGANIZATION2  "'
                - (R)09: $dateYYMMDD($$.created_at)
                - (R)10: $dateHHMM($$.created_at)
                - (R)11: '$$.delimiters.re ? $$.delimiters.re : "^"'
                - (R)12: '"00501"'
                - (R)13: $pad($.transaction_id, 'before', '0', 9)
                - (R)14: '"1"'
                - (R)15: '$$.is_prod ? "P" : "T"'
                - (R)16: '$$.delimiters.co ? $$.delimiters.co : ":"'
    # Editerra is a WYSIWYG edi mapping solution. You bring your own expertise with how the
    # edi document is supposed to be structured. That means it's perfectly fine to
    # inline a 1000B loop next to a bucket full of "header" content, like the ISA, GS, etc.
    - (S)1000B:
        components:
          - (R)NM1_RECEIVER_NAME:
              elements:
                - (R)01: '"23"'
                - (R)02: '"2"'
                - (R)03: '"ORGANIZATION2"'
                - (R)08: '"23"'
                - (R)09: '"ORGANIZATION2"'
    - (R)body:
        # use context & array to specify repeating segments
        context: $$.claims
        array: true
        components:
          - (R)2000A:
              components:
                - (R)HL_BILLING/PAY-TO_PROVIDER_HIERARCHICAL_LEVEL:
                    elements:
                      - (R)01: $hl("billing", null)
                      - (R)03: '"20"'
                      - (R)04: '"1"'
                - (R)NM1:
                    - (R)01: $.Name
```

With a payload of JSON like this:

```JSON
{
  "created_at": "2024-05-10T00:00:000Z",
  "transaction_id": "123456678",
  "is_prod": false,
  "claims": [
    {
      "Name": "John",
    },
    {
      "Name": "Jane",
    }
  ]
}
```

You'll get an edi serialization that might look like this:

```
ISA*00*          *00*          *ZZ*ORGANIZATION*ZZ*ORGANIZATION2*240510*1312*^*0501*123456678*1*T*:~
NM1*23*2*ORGANIZATION2*23*ORGANIZATION2~
HL*1**20*1~
NM1*John~
HL*2**20*1~
NM1*Jane~
```

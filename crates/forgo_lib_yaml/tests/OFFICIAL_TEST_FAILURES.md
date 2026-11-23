# Official YAML Test Suite - Failure Report

**Total Failures:** 47 out of 363 tests (12.9%)

---

## Test: 2CMS - Invalid mapping in plain multiline

**Tags:** error, mapping

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
this
 is
  invalid: x
```

---

## Test: 4JVG - Scalar value with two anchors

**Tags:** anchor, error, mapping

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
top1: &node1
  &k1 key1: val1
top2: &node2
  &v2 val2
```

---

## Test: 5LLU - Block scalar with wrong indented line after spaces only

**Tags:** error, folded, whitespace

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
block scalar: >
␣
␣␣
␣␣␣
 invalid
```

---

## Test: 8XDJ - Comment in plain multiline value

**Tags:** error, comment, scalar

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
key: word1
#  xxx
  word2
```

---

## Test: 9HCY - Need document footer before directives

**Tags:** directive, error, footer, tag, unknown-tag

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
!foo "bar"
%TAG ! tag:example.com,2000:app/
---
!foo "bar"
```

---

## Test: 9MMA - Directive by itself with no document

**Tags:** error, directive

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
%YAML 1.2
```

---

## Test: B63P - Directive without document

**Tags:** error, directive, document

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
%YAML 1.2
...
```

---

## Test: BF9H - Trailing comment in multiline plain scalar

**Tags:** comment, error, scalar

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
---
plain: a
       b # end of scalar
       c
```

---

## Test: BS4K - Comment between plain scalar lines

**Tags:** error, scalar

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
word1  # comment
word2
```

---

## Test: C2SP - Flow Mapping Key on two lines

**Tags:** error, flow, mapping

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
[23
]: 42
```

---

## Test: DK4H - Implicit key followed by newline

**Tags:** error, flow, mapping, sequence

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
---
[ key
  : value ]
```

---

## Test: DK95 - DK95-1

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
foo: "bar
————»baz"
```

---

## Test: DK95 - DK95-2

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
foo:
  a: 1
  ——»b: 2
```

---

## Test: EB22 - Missing document-end marker before directive

**Tags:** error, directive, footer

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
---
scalar1 # comment
%YAML 1.2
---
scalar2
```

---

## Test: EW3V - Wrong indendation in mapping

**Tags:** error, mapping, indent

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
k1: v1
 k2: v2
```

---

## Test: F8F9 - Spec Example 8.5. Chomping Trailing Lines

**Tags:** spec, literal, scalar, comment

**Failure Type:** Should parse but failed

**Error:**
```
Parse error: block scalar indentation error: leading empty line has 2 spaces but first content line has only 0
```

**Input YAML:**
```yaml
# Strip
 # Comments:
strip: |-
 # text
␣␣
# Clip
 # comments:

clip: |
 # text
␣
# Keep
 # comments:

keep: |+
 # text

# Trail
 # comments.
```

---

## Test: FBC9 - Allowed characters in plain scalars

**Tags:** scalar

**Failure Type:** Should parse but failed

**Error:**
```
Parse error: Invalid escape sequence: \]
```

**Input YAML:**
```yaml
safe: a!"#$%&'()*+,-./09:;<=>?@AZ[\]^_`az{|}~
     !"#$%&'()*+,-./09:;<=>?@AZ[\]^_`az{|}~
safe question mark: ?foo
safe colon: :foo
safe dash: -foo
```

---

## Test: G4RS - Spec Example 2.17. Quoted Scalars

**Tags:** spec, scalar

**Failure Type:** Should parse but failed

**Error:**
```
Parse error: orphaned content [at block end]: expected ':' after mapping key at indent 0 (missing colon?)
```

**Input YAML:**
```yaml
unicode: "Sosa did fine.\u263A"
control: "\b1998\t1999\t2000\n"
hex esc: "\x0d\x0a is \r\n"

single: '"Howdy!" he cried.'
quoted: ' # Not a ''comment''.'
tie-fighter: '|\-*-/|'
```

---

## Test: G9HC - Invalid anchor in zero indented sequence

**Tags:** anchor, error, sequence

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
---
seq:
&anchor
- a
- b
```

---

## Test: GT5M - Node anchor in sequence

**Tags:** anchor, error, sequence

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
- item1
&node
- item2
```

---

## Test: H7J7 - Node anchor not indented

**Tags:** anchor, error, indent, tag

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
key: &x
!!map
  a: b
```

---

## Test: HU3P - Invalid Mapping in plain scalar

**Tags:** error, mapping, scalar

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
key:
  word1 word2
  no: key
```

---

## Test: JKF3 - Multiline unidented double quoted block key

**Tags:** indent

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
- - "bar
bar": x
```

---

## Test: M6YH - Block sequence indentation

**Tags:** indent

**Failure Type:** Should parse but failed

**Error:**
```
Parse error: inconsistent indentation in sequence: expected indent 0, found 1
```

**Input YAML:**
```yaml
- |
 x
-
 foo: bar
-
 - 42
```

---

## Test: MUS6 - Directive variants

**Tags:** directive

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
%YAML 1.1#...
---
```

---

## Test: P2EQ - Invalid sequene item on same line as previous item

**Tags:** error, flow, mapping, sequence

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
---
- { y: z }- invalid
```

---

## Test: Q5MG - Tab at beginning of line followed by a flow mapping

**Tags:** flow, whitespace

**Failure Type:** Should parse but failed

**Error:**
```
Parse error: tabs are not allowed for indentation
```

**Input YAML:**
```yaml
———»{}
```

---

## Test: QB6E - Wrong indented multiline quoted scalar

**Tags:** double, error, indent

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
---
quoted: "a
b
c"
```

---

## Test: QLJ7 - Tag shorthand used in documents but only defined in the first

**Tags:** error, directive, tag

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
%TAG !prefix! tag:example.com,2011:
--- !prefix!A
a: b
--- !prefix!B
c: d
--- !prefix!C
e: f
```

---

## Test: R4YG - Spec Example 8.2. Block Indentation Indicator

**Tags:** spec, literal, folded, scalar, whitespace, libyaml-err, upto-1.2

**Failure Type:** Should parse but failed

**Error:**
```
Parse error: block scalar indentation error: leading empty line has 1 spaces but first content line has only 0
```

**Input YAML:**
```yaml
- |
 detected
- >
␣
␣␣
  # detected
- |1
  explicit
- >
 ——»
 detected
```

---

## Test: RXY3 - Invalid document-end marker in single quoted string

**Tags:** footer, single, error

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
---
'
...
'
```

---

## Test: S98Z - Block scalar with more spaces than first content line

**Tags:** error, folded, comment, scalar, whitespace

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
empty block scalar: >
␣
␣␣
␣␣␣
 # comment
```

---

## Test: SBG9 - Flow Sequence in Flow Mapping

**Tags:** complex-key, sequence, mapping, flow

**Failure Type:** Should parse but failed

**Error:**
```
Parse error: flow sequence not allowed as map key (use '?' for complex keys)
```

**Input YAML:**
```yaml
{a: [b, c], [d, e]: f}
```

---

## Test: SY6V - Anchor before sequence entry on same line

**Tags:** anchor, error, sequence

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
&anchor - sequence entry
```

---

## Test: W9L4 - Literal block scalar with more spaces in first line

**Tags:** error, literal, whitespace

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
---
block scalar: |
␣␣␣␣␣
  more spaces at the beginning
  are invalid
```

---

## Test: WZ62 - Spec Example 7.2. Empty Content

**Tags:** spec, flow, scalar, tag

**Failure Type:** Should parse but failed

**Error:**
```
Parse error: invalid tag format: '!!str' cannot be followed by ',' without whitespace
```

**Input YAML:**
```yaml
{
  foo : !!str,
  !!str : bar,
}
```

---

## Test: X38W - Aliases in Flow Objects

**Tags:** alias, complex-key, flow

**Failure Type:** Should parse but failed

**Error:**
```
Parse error: flow sequence not allowed as map key (use '?' for complex keys)
```

**Input YAML:**
```yaml
{ &a [a, &b b]: *b, *a : [c, *b, d]}
```

---

## Test: Y79Y - Tabs in various contexts

**Tags:** whitespace

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
foo: |
————»
bar: 1
```

---

## Test: Y79Y - Y79Y-2

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
-———»-
```

---

## Test: Y79Y - Y79Y-3

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
- ——»-
```

---

## Test: Y79Y - Y79Y-4

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
?———»-
```

---

## Test: Y79Y - Y79Y-5

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
? -
:———»-
```

---

## Test: Y79Y - Y79Y-6

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
?———»key:
```

---

## Test: Y79Y - Y79Y-7

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
? key:
:———»key:
```

---

## Test: ZCZ6 - Invalid mapping in plain single line value

**Tags:** error, mapping, scalar

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
a: b: c: d
```

---

## Test: ZVH3 - Wrong indented sequence item

**Tags:** error, sequence, indent

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
- key: value
 - item1
```

---

## Test: ZXT5 - Implicit key followed by newline and adjacent value

**Tags:** error, flow, mapping, sequence

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
[ "key"
  :value ]
```

---


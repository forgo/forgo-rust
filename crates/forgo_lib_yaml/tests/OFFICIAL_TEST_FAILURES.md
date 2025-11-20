# Official YAML Test Suite - Failure Report

**Total Failures:** 70 out of 363 tests (19.3%)

---

## Test: 236B - Invalid value after mapping

**Tags:** error, mapping

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
foo:
  bar
invalid
tree: |
+STR
 +DOC
  +MAP
   =VAL :foo
   =VAL :bar
```

---

## Test: 2CMS - Invalid mapping in plain multiline

**Tags:** error, mapping

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
this
 is
  invalid: x
tree: |
+STR
 +DOC
```

---

## Test: 4EJS - Invalid tabs as indendation in a mapping

**Tags:** error, mapping, whitespace

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
---
a:
———»b:
———»———»c: value
tree: |
+STR
 +DOC ---
  +MAP
   =VAL :a
```

---

## Test: 4HVU - Wrong indendation in Sequence

**Tags:** error, sequence, indent

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
key:
   - ok
   - also ok
  - wrong
tree: |
+STR
 +DOC
  +MAP
   =VAL :key
   +SEQ
    =VAL :ok
    =VAL :also ok
   -SEQ
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
tree: |
+STR
 +DOC
  +MAP
   =VAL :top1
   +MAP &node1
    =VAL &k1 :key1
    =VAL :val1
   -MAP
   =VAL :top2
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
tree: |
+STR
 +DOC
  +MAP
   =VAL :block scalar
```

---

## Test: 62EZ - Invalid block mapping key on same line as previous key

**Tags:** error, flow, mapping

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
---
x: { y: z }in: valid
tree: |
+STR
 +DOC ---
  +MAP
   =VAL :x
   +MAP {}
    =VAL :y
    =VAL :z
   -MAP
```

---

## Test: 6S55 - Invalid scalar at the end of sequence

**Tags:** error, mapping, sequence

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
key:
 - bar
 - baz
 invalid
tree: |
+STR
 +DOC
  +MAP
   =VAL :key
   +SEQ
    =VAL :bar
    =VAL :baz
```

---

## Test: 7LBH - Multiline double quoted implicit keys

**Tags:** error, double

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
"a\nb": 1
"c
 d": 1
tree: |
+STR
 +DOC
  +MAP
   =VAL "a\nb
   =VAL :1
```

---

## Test: 7MNF - Missing colon

**Tags:** error, mapping

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
top1:
  key1: val1
top2
tree: |
+STR
 +DOC
  +MAP
   =VAL :top1
   +MAP
    =VAL :key1
    =VAL :val1
   -MAP
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
tree: |
+STR
 +DOC
  +MAP
   =VAL :key
   =VAL :word1
```

---

## Test: 9CWY - Invalid scalar at the end of mapping

**Tags:** error, mapping, sequence

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
key:
 - item1
 - item2
invalid
tree: |
+STR
 +DOC
  +MAP
   =VAL :key
   +SEQ
    =VAL :item1
    =VAL :item2
   -SEQ
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
tree: |
+STR
 +DOC
  =VAL <!foo> "bar
```

---

## Test: 9JBA - Invalid comment after end of flow sequence

**Tags:** comment, error, flow, sequence

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
---
[ a, b, c, ]#invalid
tree: |
+STR
 +DOC ---
  +SEQ []
   =VAL :a
   =VAL :b
   =VAL :c
  -SEQ
```

---

## Test: 9MMA - Directive by itself with no document

**Tags:** error, directive

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
%YAML 1.2
tree: |
+STR
```

---

## Test: 9MQT - Scalar doc with '...' in content

**Tags:** double, scalar

**Failure Type:** Should parse but failed

**Error:**
```
Parse error: double-quoted scalar continuation line must be indented (cannot start at column 0 in block context)
```

**Input YAML:**
```yaml
--- "a
...x
b"
tree: |
+STR
 +DOC ---
  =VAL "a ...x b
 -DOC
-STR
json: |
"a ...x b"
dump: |
--- a ...x b
emit: |
--- "a ...x b"
```

---

## Test: B63P - Directive without document

**Tags:** error, directive, document

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
%YAML 1.2
...
tree: |
+STR
```

---

## Test: BD7L - Invalid mapping after sequence

**Tags:** error, mapping, sequence

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
- item1
- item2
invalid: x
tree: |
+STR
 +DOC
  +SEQ
   =VAL :item1
   =VAL :item2
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
tree: |
+STR
 +DOC ---
  +MAP
   =VAL :plain
   =VAL :a b
```

---

## Test: BS4K - Comment between plain scalar lines

**Tags:** error, scalar

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
word1  # comment
word2
tree: |
+STR
 +DOC
  =VAL :word1
 -DOC
```

---

## Test: C2SP - Flow Mapping Key on two lines

**Tags:** error, flow, mapping

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
[23
]: 42
tree: |
+STR
 +DOC
  +SEQ []
   =VAL :23
```

---

## Test: CVW2 - Invalid comment after comma

**Tags:** comment, error, flow, sequence

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
---
[ a, b, c,#invalid
]
tree: |
+STR
 +DOC ---
  +SEQ []
   =VAL :a
   =VAL :b
   =VAL :c
```

---

## Test: D49Q - Multiline single quoted implicit keys

**Tags:** error, single, mapping

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
'a\nb': 1
'c
 d': 1
tree: |
+STR
 +DOC
  +MAP
   =VAL 'a\\nb
   =VAL :1
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
tree: |
+STR
 +DOC ---
  +SEQ []
   =VAL :key
```

---

## Test: DK95 - DK95-2

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
foo:
  a: 1
  ——»b: 2
tree: |
+STR
 +DOC
  +MAP
   =VAL :foo
   +MAP
    =VAL :a
    =VAL :1
```

---

## Test: DMG6 - Wrong indendation in Map

**Tags:** error, mapping, indent

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
key:
  ok: 1
 wrong: 2
tree: |
+STR
 +DOC
  +MAP
   =VAL :key
   +MAP
    =VAL :ok
    =VAL :1
   -MAP
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
tree: |
+STR
 +DOC ---
  =VAL :scalar1
 -DOC
```

---

## Test: EW3V - Wrong indendation in mapping

**Tags:** error, mapping, indent

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
k1: v1
 k2: v2
tree: |
+STR
 +DOC
  +MAP
   =VAL :k1
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
tree: |
+STR
+DOC
 +MAP
  =VAL :strip
  =VAL |# text
  =VAL :clip
  =VAL |# text\n
  =VAL :keep
  =VAL |# text\n\n
 -MAP
-DOC
-STR
json: |
{
 "strip": "# text",
 "clip": "# text\n",
 "keep": "# text\n\n"
}
dump: |
strip: |-
 # text
clip: |
 # text
keep: |+
 # text

...
```

---

## Test: FTA2 - Single block sequence with anchor and explicit document start

**Tags:** anchor, header, sequence

**Failure Type:** Should parse but failed

**Error:**
```
Parse error: unexpected token at indent 0: Amp
```

**Input YAML:**
```yaml
--- &sequence
- a
tree: |
+STR
 +DOC ---
  +SEQ &sequence
   =VAL :a
  -SEQ
 -DOC
-STR
json: |
[
  "a"
]
dump: |
--- &sequence
- a
```

---

## Test: G7JE - Multiline implicit keys

**Tags:** error, mapping

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
a\nb: 1
c
 d: 1
tree: |
+STR
 +DOC
  +MAP
   =VAL :a\\nb
   =VAL :1
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
tree: |
+STR
 +DOC ---
  +MAP
   =VAL :seq
```

---

## Test: GDY7 - Comment that looks like a mapping key

**Tags:** comment, error, mapping

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
key: value
this is #not a: key
tree: |
+STR
 +DOC
  +MAP
   =VAL :key
   =VAL :value
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
tree: |
+STR
 +DOC
  +SEQ
   =VAL :item1
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
tree: |
+STR
 +DOC
  +MAP
   =VAL :key
   =VAL &x :
```

---

## Test: H7TQ - Extra words on %YAML directive

**Tags:** directive

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
%YAML 1.2 foo
---
tree: |
+STR
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
tree: |
+STR
 +DOC
  +MAP
   =VAL :key
```

---

## Test: JY7Z - Trailing content that looks like a mapping

**Tags:** error, mapping, double

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
key1: "quoted1"
key2: "quoted2" no key: nor value
key3: "quoted3"
tree: |
+STR
 +DOC
  +MAP
   =VAL :key1
   =VAL "quoted1
   =VAL :key2
   =VAL "quoted2
```

---

## Test: LHL4 - Invalid tag

**Tags:** error, tag

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
---
!invalid{}tag scalar
tree: |
+STR
 +DOC ---
```

---

## Test: N4JP - Bad indentation in mapping

**Tags:** error, mapping, indent, double

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
map:
  key1: "quoted1"
 key2: "bad indentation"
tree: |
+STR
 +DOC
  +MAP
   =VAL :map
   +MAP
    =VAL :key1
    =VAL "quoted1
   -MAP
```

---

## Test: NP9H - Spec Example 7.5. Double Quoted Line Breaks

**Tags:** double, spec, scalar, whitespace, upto-1.2

**Failure Type:** Should parse but failed

**Error:**
```
Parse error: double-quoted scalar continuation line must be indented (cannot start at column 0 in block context)
```

**Input YAML:**
```yaml
"folded␣
to a space,»
␣
to a line feed, or »\
 \ »non-content"
tree: |
+STR
 +DOC
  =VAL "folded to a space,\nto a line feed, or \t \tnon-content
 -DOC
-STR
json: |
"folded to a space,\nto a line feed, or \t \tnon-content"
dump: |
"folded to a space,\nto a line feed, or \t \tnon-content"
```

---

## Test: P2EQ - Invalid sequene item on same line as previous item

**Tags:** error, flow, mapping, sequence

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
---
- { y: z }- invalid
tree: |
+STR
 +DOC ---
  +SEQ
   +MAP {}
    =VAL :y
    =VAL :z
   -MAP
```

---

## Test: Q4CL - Trailing content after quoted value

**Tags:** error, mapping, double

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
key1: "quoted1"
key2: "quoted2" trailing content
key3: "quoted3"
tree: |
+STR
 +DOC
  +MAP
   =VAL :key1
   =VAL "quoted1
   =VAL :key2
   =VAL "quoted2
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
tree: |
+STR
 +DOC ---
  +MAP <tag:example.com,2011:A>
   =VAL :a
   =VAL :b
  -MAP
 -DOC
 +DOC ---
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
tree: |
+STR
 +DOC
  +SEQ
   =VAL |detected\n
   =VAL >\n\n# detected\n
   =VAL | explicit\n
   =VAL >\t\ndetected\n
  -SEQ
 -DOC
-STR
json: |
[
  "detected\n",
  "\n\n# detected\n",
  " explicit\n",
  "\t\ndetected\n"
]
dump: |
- |
  detected
- >2


  # detected
- |2
   explicit
- "\t\ndetected\n"
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
tree: |
+STR
 +DOC ---
```

---

## Test: S4GJ - Invalid text after block scalar indicator

**Tags:** error, folded

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
---
folded: > first line
  second line
tree: |
+STR
 +DOC ---
  +MAP
   =VAL :folded
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
tree: |
+STR
 +DOC
  +MAP
   =VAL :empty block scalar
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
tree: |
+STR
 +DOC
  +MAP {}
   =VAL :a
   +SEQ []
    =VAL :b
    =VAL :c
   -SEQ
   +SEQ []
    =VAL :d
    =VAL :e
   -SEQ
   =VAL :f
  -MAP
 -DOC
-STR
dump: |
a:
- b
- c
? - d
  - e
: f
```

---

## Test: SR86 - Anchor plus Alias

**Tags:** alias, error

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
key1: &a value
key2: &b *a
tree: |
+STR
 +DOC
  +MAP
   =VAL :key1
   =VAL &a :value
   =VAL :key2
```

---

## Test: SU5Z - Comment without whitespace after doublequoted scalar

**Tags:** comment, error, double, whitespace

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
key: "value"# invalid comment
tree: |
+STR
 +DOC
  +MAP
   =VAL :key
   =VAL "value
```

---

## Test: SU74 - Anchor and alias as mapping key

**Tags:** error, anchor, alias, mapping

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
key1: &alias value1
&b *alias : value2
tree: |
+STR
 +DOC
  +MAP
   =VAL :key1
   =VAL &alias :value1
```

---

## Test: SY6V - Anchor before sequence entry on same line

**Tags:** anchor, error, sequence

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
&anchor - sequence entry
tree: |
+STR
```

---

## Test: TD5N - Invalid scalar after sequence

**Tags:** error, sequence, scalar

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
- item1
- item2
invalid
tree: |
+STR
 +DOC
  +SEQ
   =VAL :item1
   =VAL :item2
```

---

## Test: U44R - Bad indentation in mapping (2)

**Tags:** error, mapping, indent, double

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
map:
  key1: "quoted1"
   key2: "bad indentation"
tree: |
+STR
 +DOC
  +MAP
   =VAL :map
   +MAP
    =VAL :key1
    =VAL "quoted1
```

---

## Test: U99R - Invalid comma in tag

**Tags:** error, tag

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
- !!str, xxx
tree: |
+STR
 +DOC
  +SEQ
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
tree: |
+STR
 +DOC ---
  +MAP
   =VAL :block scalar
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
tree: |
+STR
 +DOC
  +MAP {}
   +SEQ [] &a
    =VAL :a
    =VAL &b :b
   -SEQ
   =ALI *b
   =ALI *a
   +SEQ []
    =VAL :c
    =ALI *b
    =VAL :d
   -SEQ
  -MAP
 -DOC
-STR
dump: |
? &a
- a
- &b b
: *b
*a :
- c
- *b
- d
```

---

## Test: X4QW - Comment without whitespace after block scalar indicator

**Tags:** folded, comment, error, whitespace

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
block: ># comment
  scalar
tree: |
+STR
 +DOC
  +MAP
   =VAL :block
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
tree: |
+STR
 +DOC
  +MAP
   =VAL :foo
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
tree: |
+STR
 +DOC
  +MAP
   =VAL :a
```

---

## Test: ZL4Z - Invalid nested mapping

**Tags:** error, mapping

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
---
a: 'b': c
tree: |
+STR
 +DOC ---
  +MAP
   =VAL :a
   =VAL 'b
```

---

## Test: ZVH3 - Wrong indented sequence item

**Tags:** error, sequence, indent

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
- key: value
 - item1
tree: |
+STR
 +DOC
  +SEQ
   +MAP
    =VAL :key
    =VAL :value
   -MAP
```

---

## Test: ZXT5 - Implicit key followed by newline and adjacent value

**Tags:** error, flow, mapping, sequence

**Failure Type:** Should fail but parsed successfully

**Input YAML:**
```yaml
[ "key"
  :value ]
tree: |
+STR
 +DOC
  +SEQ []
   =VAL "key
```

---


# PostgreSQL string functions

`dbkit::func` provides typed expressions for PostgreSQL string functions. They accept literals,
generated model columns, and other expressions, so the same helper can be used in projections,
filters, and ordering.

## Practical queries

This example uses generated model columns directly.
The projection uses `sqlx::FromRow`, which requires `sqlx` as a direct dependency.

```rust
use dbkit::func::{self, IntoConcatExpr, NormalizationForm, RegexReplaceFlags, RegexSplitFlags};
use dbkit::{model, Order};

#[model(table = "articles")]
struct Article {
    #[key]
    id: i64,
    title: String,
    subtitle: Option<String>,
    body: String,
    delimiter: String,
    pattern: String,
    replacement: String,
    codepoint: i32,
}

#[derive(sqlx::FromRow)]
struct ArticleText {
    display_title: String,
    heading: String,
    parts: Vec<String>,
    last_part: String,
    captures: Option<Vec<Option<String>>>,
    cleaned_body: String,
    words: Vec<String>,
    normalized_title: String,
    first_codepoint: i32,
    character: String,
}

let normalized_title = func::lower(func::trim(Article::title));
let _matching_articles = Article::query()
    .filter(normalized_title.clone().eq("rust and postgres"))
    .order_by(Order::asc(normalized_title));

let heading = func::concat_with_separator(
    " - ",
    [
        Article::title.into_concat_expr(),
        Article::subtitle.into_concat_expr(),
    ],
);
let cleaned_body = func::regex_replace(
    Article::body,
    Article::pattern,
    Article::replacement,
    RegexReplaceFlags::GLOBAL | RegexReplaceFlags::CASE_INSENSITIVE,
);
let _article_text = Article::query()
    .select_only()
    .column_as(func::title_case(Article::title), "display_title")
    .column_as(heading, "heading")
    .column_as(func::split(Article::body, Article::delimiter), "parts")
    .column_as(func::split_part(Article::body, Article::delimiter, -1), "last_part")
    .column_as(func::regex_captures(Article::body, Article::pattern), "captures")
    .column_as(cleaned_body.clone(), "cleaned_body")
    .column_as(
        func::regex_split(cleaned_body, r"\s+", RegexSplitFlags::empty()),
        "words",
    )
    .column_as(func::normalize(Article::title, NormalizationForm::Nfc), "normalized_title")
    .column_as(func::first_codepoint(Article::title), "first_codepoint")
    .column_as(func::from_codepoint(Article::codepoint), "character")
    .filter(func::regex_is_match(Article::body, Article::pattern).eq(true))
    .into_model::<ArticleText>();

// PostgreSQL 18 and UTF8 only.
let _unicode_check = Article::query()
    .filter(func::is_unicode_assigned(Article::body).eq(true))
    .order_by(Order::asc(func::case_fold(Article::title)));
```

Rust strings and other runtime values in these calls become bind parameters. This includes regex
patterns, replacements, separators, and delimiters. Function names are fixed by dbkit, regex flags
come from closed bitflag types, and normalization forms come from `NormalizationForm`. Do not pass
SQL fragments where a value is expected. PostgreSQL validates regex syntax when it executes the
query.

## 1. Case conversion and trimming

| dbkit API | PostgreSQL mapping | Behavior |
| --- | --- | --- |
| `upper(text)` | `UPPER(text)` | Converts text to uppercase using the database locale. |
| `lower(text)` | `LOWER(text)` | Converts text to lowercase using the database locale. |
| `trim(text)` | `TRIM(text)` | Removes spaces from both ends. |
| `trim_chars(text, chars)` | `TRIM(BOTH chars FROM text)` | Removes the longest span made from the `chars` set at both ends. |
| `trim_start(text)` | `TRIM(LEADING FROM text)` | Removes leading spaces. |
| `trim_start_chars(text, chars)` | `TRIM(LEADING chars FROM text)` | Removes a leading span made from the `chars` set. |
| `trim_end(text)` | `TRIM(TRAILING FROM text)` | Removes trailing spaces. |
| `trim_end_chars(text, chars)` | `TRIM(TRAILING chars FROM text)` | Removes a trailing span made from the `chars` set. |

The `chars` argument is a set of characters, not a literal prefix or suffix.

## 2. Length and search

| dbkit API | PostgreSQL mapping | Behavior |
| --- | --- | --- |
| `char_length(text)` | `CHAR_LENGTH(text)` | Counts characters. |
| `byte_length(text)` | `OCTET_LENGTH(text)` | Counts encoded bytes. |
| `bit_length(text)` | `BIT_LENGTH(text)` | Returns eight times the encoded byte length. |
| `position(text, substring)` | `STRPOS(text, substring)` | Returns the first 1-based position, or zero when absent. |
| `starts_with(text, prefix)` | `STARTS_WITH(text, prefix)` | Tests an exact, case-sensitive prefix. Requires PostgreSQL 11 or newer. |

Character and byte counts differ for multibyte text. For example, one Unicode character can occupy
several bytes in `UTF8`.

## 3. Extraction and sizing

| dbkit API | PostgreSQL mapping | Behavior |
| --- | --- | --- |
| `left(text, count)` | `LEFT(text, count)` | Returns the first `count` characters. A negative count omits characters from the end. |
| `right(text, count)` | `RIGHT(text, count)` | Returns the last `count` characters. A negative count omits characters from the start. |
| `substring(text, start, count)` | `SUBSTRING(text, start, count)` | Returns up to `count` characters from the 1-based start. PostgreSQL rejects a negative count. |
| `repeat(text, count)` | `REPEAT(text, count)` | Repeats text. A non-positive count returns an empty string. |
| `pad_start(text, length, fill)` | `LPAD(text, length, fill)` | Pads on the left by cycling `fill`, or truncates on the right. |
| `pad_end(text, length, fill)` | `RPAD(text, length, fill)` | Pads on the right by cycling `fill`, or truncates on the right. |

## 4. Text transformation

| dbkit API | PostgreSQL mapping | Behavior |
| --- | --- | --- |
| `title_case(text)` | `INITCAP(text)` | Uppercases the first letter of each alphanumeric word and lowercases the rest. |
| `replace(text, from, to)` | `REPLACE(text, from, to)` | Replaces every exact occurrence of `from`. |
| `replace_range(text, replacement, start, count)` | `OVERLAY(text, replacement, start, count)` | Replaces `count` characters from the 1-based start. |
| `translate_chars(text, from, to)` | `TRANSLATE(text, from, to)` | Maps characters by position. Extra characters in `from` are deleted. |
| `reverse(text)` | `REVERSE(text)` | Reverses characters. |

`replace` works on substrings. `translate_chars` works on individual characters. `replace_range`
uses PostgreSQL's callable `OVERLAY` form, which is equivalent to `OVERLAY(text PLACING replacement
FROM start FOR count)`.

## 5. Composition and splitting

| dbkit API | PostgreSQL mapping | Behavior |
| --- | --- | --- |
| `concat(values)` | `CONCAT(values...)` | Concatenates values and ignores NULL items. The result type is `String`. |
| `concat_with_separator(separator, values)` | `CONCAT_WS(separator, values...)` | Inserts the separator and ignores NULL items. A NULL separator returns NULL. |
| `split(text, delimiter)` | `STRING_TO_ARRAY(text, delimiter)` | Returns `Vec<String>`. A NULL delimiter splits into characters; an empty delimiter returns one field. |
| `split_part(text, delimiter, index)` | `SPLIT_PART(text, delimiter, index)` | Returns the indexed field. Indexes are 1-based and zero is an error. |

`split_part` returns an empty string when the index is out of range. Negative indexes count from the
end on PostgreSQL 14 or newer.

When a concat list mixes required and optional columns, call `.into_concat_expr()` on every item so
Rust can put them in one array. PostgreSQL ignores NULL items. `concat_with_separator` returns NULL
only when the separator is NULL.

The two-argument `split` helper cannot create NULL array elements because it does not expose
PostgreSQL's optional `null_string` argument. A nullable source produces `Option<Vec<String>>`.

## 6. Regex inspection

| dbkit API | PostgreSQL mapping | Behavior |
| --- | --- | --- |
| `regex_is_match(text, pattern)` | `REGEXP_LIKE(text, pattern)` | Tests whether a POSIX regex matches anywhere. Requires PostgreSQL 15 or newer. |
| `regex_count(text, pattern)` | `REGEXP_COUNT(text, pattern)` | Counts non-overlapping matches. Requires PostgreSQL 15 or newer. |
| `regex_position(text, pattern)` | `REGEXP_INSTR(text, pattern)` | Returns the first 1-based match position, or zero. Requires PostgreSQL 15 or newer. |
| `regex_captures(text, pattern)` | `REGEXP_MATCH(text, pattern)` | Returns captures from the first match. Requires PostgreSQL 10 or newer. |
| `regex_extract(text, pattern)` | `REGEXP_SUBSTR(text, pattern)` | Returns the first whole match. Requires PostgreSQL 15 or newer. |

`regex_captures` returns `Option<Vec<Option<String>>>`. No match or a NULL input produces the outer
`None`. A pattern without capture groups produces one element containing the whole match. With
groups, each array element corresponds to one capture group. An unmatched optional group is `None`;
an empty match is `Some("")`.

These inspection helpers use PostgreSQL's default regex behavior and do not accept flag arguments.
When needed, an ARE inline option such as `(?i)` can make a pattern case-insensitive.

## 7. Regex transformation

| dbkit API | PostgreSQL mapping | Behavior |
| --- | --- | --- |
| `RegexReplaceFlags::empty()` | empty PostgreSQL flags string | Replaces only the first match, case-sensitively. |
| `RegexReplaceFlags::CASE_INSENSITIVE` | `i` | Enables case-insensitive matching. |
| `RegexReplaceFlags::GLOBAL` | `g` | Replaces every match. Combine it with `CASE_INSENSITIVE` for `gi`. |
| `RegexSplitFlags::empty()` | empty PostgreSQL flags string | Splits case-sensitively. |
| `RegexSplitFlags::CASE_INSENSITIVE` | `i` | Splits case-insensitively. |
| `regex_replace(text, pattern, replacement, flags)` | `REGEXP_REPLACE(text, pattern, replacement, flags)` | Replaces the first match by default and returns the source unchanged when there is no match. |
| `regex_split(text, pattern, flags)` | `REGEXP_SPLIT_TO_ARRAY(text, pattern, flags)` | Splits around matches into `Vec<String>`. |

`regex_replace` supports PostgreSQL replacement backreferences `\1` through `\9`, `\&` for the
whole match, and `\\` for a literal backslash. `regex_split` ignores zero-length matches at the
start or end and immediately after a previous match, matching PostgreSQL behavior.

## 8. Unicode and code points

| dbkit API | PostgreSQL mapping | Behavior |
| --- | --- | --- |
| `NormalizationForm::{Nfc, Nfd, Nfkc, Nfkd}` | `NFC`, `NFD`, `NFKC`, `NFKD` | Selects the closed normalization-form token used by `normalize`. |
| `normalize(text, form)` | `NORMALIZE(text, form)` | Normalizes Unicode text. Requires PostgreSQL 13 or newer and `UTF8`. |
| `first_codepoint(text)` | `ASCII(text)` | Returns the first character's Unicode code point in `UTF8`, or zero for empty text. |
| `from_codepoint(value)` | `CHR(value)` | Returns the character for a code point. PostgreSQL rejects zero and invalid values. |
| `to_ascii(text)` | `TO_ASCII(text)` | Removes accents when converting supported encodings to ASCII. |
| `case_fold(text)` | `CASEFOLD(text)` | Applies collation-dependent Unicode case folding. Requires PostgreSQL 18 or newer and `UTF8`. |
| `is_unicode_assigned(text)` | `UNICODE_ASSIGNED(text)` | Tests whether every character has an assigned Unicode code point. Requires PostgreSQL 18 or newer and `UTF8`. |

`to_ascii` supports `LATIN1`, `LATIN2`, `LATIN9`, and `WIN1250` database encodings. It does not
support a `UTF8` database. In non-UTF8 multibyte encodings, `ASCII` and `CHR` accept only ASCII
characters or codes.

`case_fold` can change string length and may change normalization. Its result depends on the
collation. PostgreSQL's `libc` provider currently folds like `lower`; `PG_UNICODE_FAST` follows
Unicode Default Caseless Matching.

## NULL handling and collation

Most PostgreSQL string functions return NULL when any text input is NULL. When projecting a result
based on a nullable model column, use the matching optional type in the `FromRow` struct. Text
results use `Option<String>`, lengths and positions use `Option<i32>`, boolean checks use
`Option<bool>`, and split results use `Option<Vec<String>>`.

The main exceptions are `concat`, which ignores NULL items, and `concat_with_separator`, which
ignores NULL items but returns NULL for a NULL separator. `split` treats a NULL delimiter as a
request to split into characters. Regex captures and extraction also use optional result fields
because no match returns NULL.

`upper`, `lower`, `title_case`, and `case_fold` depend on the database locale or expression
collation. POSIX regex character classes and case-insensitive matching can also vary with collation;
POSIX regex functions do not support nondeterministic collations.

See PostgreSQL's [string function reference](https://www.postgresql.org/docs/18/functions-string.html)
and [POSIX regular expression reference](https://www.postgresql.org/docs/18/functions-matching.html#FUNCTIONS-POSIX-REGEXP).

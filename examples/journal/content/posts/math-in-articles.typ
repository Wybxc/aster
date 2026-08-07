#import "/templates/content.typ": post

#show: post.with(
  title: "Write mathematics in journal articles",
  description: "Add accessible mathematical notation directly to an article.",
  author: "Papertrail Editors",
  date: "2024-09-08T20:58:52Z",
  modified: "2025-03-22T09:25:46Z",
  tags: ("docs",),
)

Technical articles often need notation that is clearer as mathematics than as
plain text. Typst can express inline and displayed equations in the same source
as the surrounding prose.

= Inline notation

Use inline mathematics when a symbol is part of a sentence. For example, the
identity $e^(i pi) + 1 = 0$ remains aligned with the baseline and receives
MathML in HTML output.

= Displayed equations

Set longer derivations apart from the paragraph:

$ integral_0^infinity e^(-x^2) dif x = sqrt(pi) / 2 $

The source stays concise:

```typ
$ integral_0^infinity e^(-x^2) dif x = sqrt(pi) / 2 $
```

= Explain the result

Notation should support the article rather than replace its explanation. Define
unfamiliar symbols in prose and add surrounding context for readers using assistive
technology.

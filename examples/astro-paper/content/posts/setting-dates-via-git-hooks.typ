#import "/templates/content.typ": post

#show: post.with(
  title: "How to use Git Hooks to set Created and Modified Dates",
  description: "Use a small Git hook when date metadata should follow committed changes.",
  author: "Simon Smale",
  date: "2024-01-03T20:40:08Z",
  modified: "2024-01-08T18:59:05Z",
  canonical: "https://smale.codes/posts/setting-dates-via-git-hooks/",
  tags: ("docs", "FAQ"),
)

Publication dates describe when an article first became public. Modification
dates should change only when an edit materially affects what readers see.

= Decide what to automate

A hook can find staged post files and update their modified field, but it should
not rewrite every article on every commit. Keep the rule narrow and make its
behavior visible in the diff.

```sh
git diff --cached --name-only -- 'content/posts/*.typ'
```

= Keep the hook in the repository

Store the script in a tracked `hooks/` directory and configure each development
checkout to use it. This avoids hiding important publishing behavior in one
person's local `.git/hooks` directory.

= Treat dates as editorial data

Automation is a convenience rather than an authority. Authors should still be
able to preserve an intentional date when correcting spelling or formatting.

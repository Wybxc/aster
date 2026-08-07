#metadata((
  title: "Relay 1.8: faster where it matters",
  date: "2022-06-01",
  version: "1.8",
  description: "Faster navigation, incremental updates, and more reliable reconnects.",
  image: "/assets/release-1-8.jpg",
)) <aster-frontmatter>

= Faster where it matters

#html.img(src: "/assets/release-1-8.jpg", alt: "A laptop displaying a colorful burst")

This release focuses on the pauses people feel during ordinary work: opening a
large project, switching views, and reconnecting after sleep.

== New Features & Enhancements

- *Incremental project loading:* Recent work becomes interactive first.
- *Local view cache:* Switching between saved views no longer refetches data.
- *Connection status:* Sync state is visible without interrupting the task.

== Bug Fixes

- Fixed stale counters after moving several tasks quickly.
- Restored pending uploads after a reconnect.
- Reduced memory use for long activity histories.

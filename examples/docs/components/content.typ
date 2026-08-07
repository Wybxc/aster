#import "icons.typ": info-icon, link-icon, spark-icon, warning-icon

// ---------------------------------------------------------------------------
// Heading ids and tables of contents
// ---------------------------------------------------------------------------

/// Extract the plain text of a heading body for slug generation.
#let heading-text(content) = {
  if type(content) == str {
    content
  } else if repr(content) == "[ ]" {
    " "
  } else {
    let fields = content.fields()
    let text = fields.at("text", default: none)
    if text != none {
      text
    } else {
      let children = fields.at("children", default: none)
      if children != none {
        children.map(heading-text).join("")
      } else {
        let body = fields.at("body", default: none)
        if body != none { heading-text(body) } else { "" }
      }
    }
  }
}

/// Derive an HTML fragment id from a heading title.
///
/// The slug keeps letters, numbers, spaces, underscores, and hyphens, and
/// collapses runs of spaces and separators into single hyphens. CJK and other
/// Unicode letters are preserved so anchors stay readable.
#let heading-id(title) = {
  let slug = lower(title)
    .replace(regex("[^\\p{L}\\p{N} _-]"), "")
    .replace(regex("[ _]+"), "-")
    .replace(regex("-+"), "-")
  slug.trim(regex("[- ]"))
}

/// Render a heading element with a generated id and anchor link.
///
/// The heading carries a `<toc-heading>` label so `collect-toc` can find it.
/// The `show heading` rule in `templates/docs.typ` routes native heading
/// markup through this function.
#let heading-element(body, id, level) = [
  #html.div(class: "sl-heading-wrapper level-h" + str(level))[
    #html.elem("h" + str(level), attrs: (id: id))[#body] <toc-heading>
    #html.a(class: "sl-anchor-link", href: "#" + id, aria-label: "Link to this section")[#link-icon]
  ]
]

/// Collect the headings of the rendered document and pass them to `use-toc`.
///
/// `query` observes the complete document regardless of the call site, so the
/// collected entries can be rendered anywhere in a page template. The callback
/// receives the entries as an array of `(id, title, level)` dictionaries and
/// must return content.
#let collect-toc(use-toc) = context {
  use-toc(query(<toc-heading>).map(heading => {
    let attrs = heading.attrs
    (
      id: attrs.at("id", default: none),
      title: heading-text(heading.body),
      level: int(heading.tag.slice(1)),
    )
  }))
}

#metadata(
  ```css
  .sl-heading-wrapper {
    --anchor-size: 0.8275em;
    --anchor-gap: 0.3em;
    line-height: 1.2;
  }

  .sl-heading-wrapper > :first-child {
    display: inline;
    padding-inline-end: calc(var(--anchor-size) + var(--anchor-gap));
  }

  .sl-anchor-link {
    position: relative;
    display: inline-flex;
    margin-inline-start: calc(-1 * var(--anchor-size));
    color: var(--sl-color-text-accent);
    text-decoration: none;
  }

  .sl-anchor-link .sl-icon {
    width: var(--anchor-size);
    height: var(--anchor-size);
    translate: 0 0.16em;
  }

  @media (hover: hover) {
    .sl-anchor-link {
      opacity: 0;
    }

    .sl-anchor-link:focus,
    .sl-heading-wrapper:hover .sl-anchor-link {
      opacity: 1;
    }
  }
  ```
) <aster-style>

#let callout(body, kind: "note", title: none) = {
  let title = if title == none {
    (note: "Note", tip: "Tip", caution: "Caution", danger: "Danger").at(kind)
  } else {
    title
  }
  let icon = if kind == "tip" { spark-icon } else if kind == "note" { info-icon } else { warning-icon }
  [
    #metadata(
      ```css
      .sl-callout {
        --callout-border: var(--sl-color-blue);
        --callout-accent: var(--sl-color-blue-high);
        --callout-bg: var(--sl-color-blue-low);
        border-inline-start: 0.25rem solid var(--callout-border);
        padding: 1rem;
        background: var(--callout-bg);
        color: var(--sl-color-white);
      }

      .sl-callout--tip {
        --callout-border: var(--sl-color-purple);
        --callout-accent: var(--sl-color-purple-high);
        --callout-bg: var(--sl-color-purple-low);
      }

      .sl-callout--caution {
        --callout-border: var(--sl-color-orange);
        --callout-accent: var(--sl-color-orange-high);
        --callout-bg: var(--sl-color-orange-low);
      }

      .sl-callout--danger {
        --callout-border: var(--sl-color-red);
        --callout-accent: var(--sl-color-red-high);
        --callout-bg: var(--sl-color-red-low);
      }

      .sl-callout__title {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        color: var(--callout-accent);
        font-weight: 600;
        line-height: 1.2;
      }

      .sl-callout__title .sl-icon {
        width: 1.25rem;
        height: 1.25rem;
      }

      .sl-callout__content {
        margin-top: 0.5rem;
      }

      .sl-callout__content > * + * {
        margin-top: 0.65rem;
      }
      ```
    ) <aster-style>
    #html.aside(class: "sl-callout sl-callout--" + kind)[
      #html.div(class: "sl-callout__title")[#icon #title]
      #html.div(class: "sl-callout__content")[#body]
    ]
  ]
}

#let card-grid(items) = [
  #metadata(
    ```css
    .card-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(min(16rem, 100%), 1fr));
      gap: 1rem;
      padding: 0 !important;
      list-style: none;
    }

    .card-grid > li {
      min-width: 0;
    }

    .sl-card {
      height: 100%;
      border: 1px solid var(--sl-color-gray-5);
      border-radius: 0.5rem;
      padding: 1rem;
      background: var(--sl-color-bg-nav);
      box-shadow: var(--sl-shadow-md);
    }

    .sl-card h3 {
      margin: 0;
      color: var(--sl-color-white);
      font-size: 1.125rem;
      font-weight: 600;
    }

    .sl-card h3 a {
      color: inherit;
      text-decoration: none;
    }

    .sl-card h3 a::after {
      content: " ->";
      color: var(--sl-color-text-accent);
    }

    .sl-card p {
      margin-top: 0.5rem;
      color: var(--sl-color-gray-3);
      font-size: 0.875rem;
    }
    ```
  ) <aster-style>
  #html.ul(class: "card-grid")[
    #for item in items {
      html.li[
        #html.article(class: "sl-card")[
          #html.h3[
            #if item.at("href", default: none) == none {
              item.title
            } else {
              html.a(href: item.href)[#item.title]
            }
          ]
          #html.p[#item.body]
        ]
      ]
    }
  ]
]

#let steps(items) = [
  #metadata(
    ```css
    .sl-steps {
      padding-inline-start: 2.75rem !important;
      counter-reset: steps;
      list-style: none;
    }

    .sl-steps > li {
      position: relative;
      min-height: 3rem;
      padding-bottom: 1.5rem;
      counter-increment: steps;
    }

    .sl-steps > li + li {
      margin-top: 0;
    }

    .sl-steps > li::before {
      content: counter(steps);
      position: absolute;
      left: -2.75rem;
      display: grid;
      width: 1.75rem;
      height: 1.75rem;
      place-items: center;
      border-radius: 50%;
      background: var(--sl-color-accent-low);
      color: var(--sl-color-text-accent);
      font-size: 0.8125rem;
      font-weight: 700;
    }

    .sl-steps > li:not(:last-child)::after {
      content: "";
      position: absolute;
      top: 2rem;
      bottom: 0.25rem;
      left: -1.9rem;
      border-left: 1px solid var(--sl-color-gray-5);
    }

    .sl-steps h3 {
      margin: 0 0 0.5rem;
      font-size: 1.125rem;
    }
    ```
  ) <aster-style>
  #html.ol(class: "sl-steps")[
    #for item in items {
      html.li[
        #html.h3[#item.title]
        #item.body
      ]
    }
  ]
]

#let tabs(id, items) = [
  #metadata(
    ```css
    .sl-tabs {
      border: 1px solid var(--sl-color-gray-5);
      border-radius: 0.5rem;
      overflow: hidden;
    }

    .sl-tab-list {
      display: flex;
      gap: 0.25rem;
      overflow-x: auto;
      border-bottom: 1px solid var(--sl-color-gray-5);
      padding: 0.35rem 0.5rem 0;
      background: var(--sl-color-bg-nav);
    }

    .sl-tab-list button {
      flex: none;
      border: 0;
      border-bottom: 2px solid transparent;
      padding: 0.55rem 0.75rem;
      background: transparent;
      color: var(--sl-color-gray-3);
      cursor: pointer;
    }

    .sl-tab-list button[aria-selected="true"] {
      border-bottom-color: var(--sl-color-accent);
      color: var(--sl-color-white);
    }

    .sl-tab-panel {
      padding: 1rem;
    }

    .sl-tab-panel[hidden] {
      display: none;
    }
    ```
  ) <aster-style>
  #metadata(
    ```js
    document.querySelectorAll(".sl-tabs").forEach((tabs) => {
      const buttons = [...tabs.querySelectorAll('[role="tab"]')];
      const panels = [...tabs.querySelectorAll('[role="tabpanel"]')];
      buttons.forEach((button, index) => {
        button.addEventListener("click", () => {
          buttons.forEach((item, itemIndex) => {
            const active = itemIndex === index;
            item.setAttribute("aria-selected", String(active));
            item.tabIndex = active ? 0 : -1;
            panels[itemIndex].hidden = !active;
          });
        });
      });
    });
    ```
  ) <aster-script>
  #html.div(class: "sl-tabs", id: id)[
    #html.elem("div", attrs: (class: "sl-tab-list", role: "tablist"))[
      #for (index, item) in items.enumerate() {
        html.elem("button", attrs: (
          id: id + "-tab-" + str(index),
          type: "button",
          role: "tab",
          "aria-controls": id + "-panel-" + str(index),
          "aria-selected": if index == 0 { "true" } else { "false" },
          tabindex: if index == 0 { "0" } else { "-1" },
        ))[#item.label]
      }
    ]
    #for (index, item) in items.enumerate() {
      let attrs = (
        class: "sl-tab-panel",
        id: id + "-panel-" + str(index),
        role: "tabpanel",
        "aria-labelledby": id + "-tab-" + str(index),
      )
      if index != 0 { attrs.insert("hidden", "") }
      html.elem("div", attrs: attrs)[#item.body]
    }
  ]
]

#import "/lib.typ": adjacent-docs, aster-version, settings
#import "/components/header.typ": header
#import "/components/pagination.typ": pagination
#import "/components/sidebar.typ": sidebar
#import "/components/toc.typ": mobile-table-of-contents, table-of-contents

#let docs-page(entry) = {
  let meta = entry.metadata()
  let toc = meta.at("toc", default: ())
  let title = meta.title + " | " + settings.site.title
  let generator = if aster-version == none { "Aster" } else { "Aster " + aster-version }
  [
    #metadata(
      ```css
      .main-frame {
        min-height: 100vh;
        padding-top: var(--sl-nav-height);
        padding-left: var(--sl-sidebar-width);
      }

      .content-shell {
        width: min(100%, calc(var(--sl-content-width) + var(--sl-toc-width) + 5rem));
        margin-inline: auto;
        padding: 2.5rem var(--sl-content-pad-x) 4rem;
      }

      .content-grid {
        display: grid;
        grid-template-columns: minmax(0, var(--sl-content-width)) var(--sl-toc-width);
        align-items: start;
        justify-content: center;
        gap: 3rem;
      }

      .doc-panel {
        min-width: 0;
      }

      .doc-header {
        margin-bottom: 2rem;
        padding-bottom: 1.5rem;
        border-bottom: 1px solid var(--sl-color-hairline-light);
      }

      .doc-header h1 {
        color: var(--sl-color-white);
        font-size: 2.625rem;
        font-weight: 600;
        line-height: 1.2;
        letter-spacing: 0;
      }

      .doc-header p {
        margin-top: 0.5rem;
        color: var(--sl-color-gray-3);
        font-size: 1.125rem;
        line-height: 1.6;
      }

      .doc-footer {
        display: flex;
        flex-wrap: wrap;
        justify-content: space-between;
        gap: 0.75rem;
        margin-top: 3rem;
        border-top: 1px solid var(--sl-color-hairline-light);
        padding-top: 1rem;
        color: var(--sl-color-gray-3);
        font-size: 0.8125rem;
      }

      .doc-footer a {
        color: var(--sl-color-gray-2);
      }

      @media (max-width: 71.99rem) {
        .content-shell {
          width: min(100%, calc(var(--sl-content-width) + 2 * var(--sl-content-pad-x)));
          padding-top: 0;
        }

        .content-grid {
          display: block;
        }

        .doc-panel {
          padding-top: 2.5rem;
        }
      }

      @media (max-width: 49.99rem) {
        .main-frame {
          padding-left: 0;
        }

        .content-shell {
          width: 100%;
          margin: 0;
        }

        .doc-panel {
          padding-top: 2rem;
        }

        .doc-header h1 {
          font-size: 2rem;
        }
      }

      @media print {
        .main-frame {
          padding: 0;
        }

        .content-shell,
        .content-grid {
          display: block;
          width: 100%;
          padding: 0;
        }
      }
      ```
    ) <aster-style>
    #html.html(lang: settings.site.language)[
      #html.head[
        #html.meta(charset: "utf-8")
        #html.meta(name: "viewport", content: "width=device-width, initial-scale=1")
        #html.meta(name: "generator", content: generator)
        #html.meta(name: "description", content: meta.description)
        #html.meta(name: "theme-color", content: "")
        #html.title(title)
        #html.link(rel: "icon", type: "image/svg+xml", href: "/assets/logo.svg")
        #html.script(src: "/scripts/theme-init.js")
        #html.link(rel: "stylesheet", href: "/styles/base.css")
      ]
      #html.body[
        #header()
        #sidebar(entry.id)
        #html.main(class: "main-frame", id: "main-content")[
          #if toc.len() > 0 { mobile-table-of-contents(toc) }
          #html.div(class: "content-shell")[
            #html.div(class: "content-grid")[
              #html.div(class: "doc-panel")[
                #html.article(class: "sl-markdown-content")[
                  #html.header(class: "doc-header")[
                    #html.h1[#meta.title]
                    #html.p[#meta.description]
                  ]
                  #entry.render()
                ]
                #html.footer(class: "doc-footer print-hidden")[
                  #html.a(href: settings.docs.edit-base + entry.id + ".typ")[Edit this page]
                  #html.span[Built with Aster and Typst]
                ]
                #pagination(adjacent-docs(entry.id))
              ]
              #if toc.len() > 0 { table-of-contents(toc) }
            ]
          ]
        ]
      ]
    ]
  ]
}

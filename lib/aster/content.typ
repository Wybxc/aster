// Aster Content Collections — public interface
//
// Place this file in your project at lib/aster/content.typ and import:
//   #import "/lib/aster/content.typ": get-collection, get-entry, render

#let _state = sys.inputs.at("_aster")
#assert(
  _state.protocol == 1,
  message: "unsupported Aster content protocol — this version of " +
    "content.typ is incompatible with the Aster binary",
)

#let _collections = _state.collections

// Return all entries in a collection as an array, sorted by id.
#let get-collection(name) = {
  _collections
    .at(name, default: (:))
    .pairs()
    .map(pair => pair.last())
    .sorted(key: entry => entry.id)
}

// Return a single entry by collection name and id.
// Returns `none` when not found.
#let get-entry(collection, id) = {
  _collections
    .at(collection, default: (:))
    .at(id, default: none)
}

// Render an entry's body by reconstructing HTML elements.
#let render-nodes(nodes) = {
  for node in nodes {
    if node.kind == "text" {
      node.value
    } else if node.kind == "element" {
      if node.void {
        html.elem(node.tag, attrs: node.attrs)
      } else {
        html.elem(node.tag, attrs: node.attrs)[
          #render-nodes(node.children)
        ]
      }
    }
  }
}

// Render a content entry to HTML content.
#let render(entry) = render-nodes(entry.rendered)

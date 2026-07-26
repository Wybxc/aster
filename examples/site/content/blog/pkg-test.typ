#import "@preview/example:0.1.0": add, mul

= Package Test

This entry tests importing a third-party Typst package (`@preview/example`).

Functions from the package: #add(3, 4) = #add(3, 4), #mul(2, 5) = #mul(2, 5).

#html.html({
  html.p[
    Package math: #add(3, 4) = #add(3, 4), #mul(2, 5) = #mul(2, 5).
  ]
})
## Preliminaries

::: {#def-value}
A *value* is a term that cannot step.
:::

::: {#lem-progress}
A closed, well-typed term is a value or can step.
:::

::: {#thm-safety}
Well-typed terms do not get stuck.
:::

@thm-safety follows from @lem-progress and the definition of @def-value.

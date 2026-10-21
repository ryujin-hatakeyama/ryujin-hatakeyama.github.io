## Conservativity

::: {#def-conservativity name="定義的拡張の保守性 (conservativity of definitional extension)"}
An extension $T'$ of $T$ by definitions is **conservative** when every formula of the language of $T$ provable in $T'$ is provable in $T$.

The same is said of a single definition.
:::

::: {#prp-conservativity name="Conservativity of Definitional Extension"}
Every definitional extension is conservative:

- for formulas, and
- for sequents.
:::

::: {.proof of="prp-conservativity"}
By induction on derivations,
$$
\vdash_{T'} \varphi \implies \vdash_T \varphi^{*}.
$$
:::

::: {#thm-plain}
Without a name.
:::

::: {#prp-long name="A rather long name for a proposition, long enough to wrap onto a second line on a narrow screen"}
Short.
:::

::: {#lem-escape name="<b>&amp; \"quoted\"</b>"}
Escaped.
:::

By @prp-conservativity and @def-conservativity; see also @prp-long.

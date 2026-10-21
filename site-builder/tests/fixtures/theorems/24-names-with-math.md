## Extensions

::: {#def-relation name="The relation $R \subseteq X \times X$"}
A relation on $X$.

It is a set of pairs.
:::

::: {#prp-extension name="Conservativity of $T'$ over $T$"}
Every sentence in the language of $T$ provable in $T'$ is already provable in $T$:
$$
T' \vdash \varphi \implies T \vdash \varphi.
$$
:::

::: {.proof of="prp-extension"}
By @def-relation.
:::

::: {#cor-extension name="Conservativity of $T'$"}
The result follows from @prp-extension.
:::

::: {.proof of="cor-extension"}
Immediate.
:::

::: {#thm-mixed name="$\lambda$計算の合流性 (confluence of the $\lambda$-calculus)"}
簡約は合流する。
:::

::: {#lem-escape name="A \"quoted\" {brace} it's \\$5 and $\{x\}$"}
Escapes kept.
:::

::: {#cor-plain}
Without a name.
:::

::: {#cor-aside .unnumbered name="A corollary set apart"}
Not numbered.
:::

::: {#lem-broken name="Broken $\frac{1}{$ stays"}
Its name stays, as written.
:::

See @cor-extension, @thm-mixed, and @cor-plain.

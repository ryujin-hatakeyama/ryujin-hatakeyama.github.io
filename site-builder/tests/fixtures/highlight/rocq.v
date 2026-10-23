(* Addition on the naturals is commutative. *)
Require Import Arith.

Notation "x ≤ y" := (le x y) (at level 70).

Theorem add_comm' : forall n m : nat, n + m = m + n.
Proof.
  intros n m.
  induction n as [| n' IH].
  - simpl. rewrite Nat.add_0_r. reflexivity.
  - simpl. rewrite IH. rewrite Nat.add_succ_r. reflexivity.
Qed.

Fixpoint double (n : nat) : nat :=
  match n with
  | O => O
  | S n' => S (S (double n'))
  end.

Lemma double_plus : forall n, double n = n + n.
Proof. induction n; simpl; [reflexivity | rewrite IHn, <- plus_n_Sm; reflexivity]. Qed.

(* Terms of the untyped lambda calculus, with de Bruijn indices. *)
module Term = struct
  type t =
    | Var of int
    | Lam of t
    | App of t * t

  let rec shift d c = function
    | Var k -> if k >= c then Var (k + d) else Var k
    | Lam body -> Lam (shift d (c + 1) body)
    | App (f, x) -> App (shift d c f, shift d c x)

  let to_string t =
    let rec go = function
      | Var k -> string_of_int k
      | Lam b -> "λ." ^ go b
      | App (f, x) -> Printf.sprintf "(%s %s)" (go f) (go x)
    in
    go t
end

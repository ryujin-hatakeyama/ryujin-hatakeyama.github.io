(* The power function, staged: the exponent is known now, the base later. *)
let rec spower (n : int) (x : int code) : int code =
  if n = 0 then .<1>.
  else if n mod 2 = 0 then .<let y = .~(spower (n / 2) x) in y * y>.
  else .<.~x * .~(spower (n - 1) x)>.

let power7 : (int -> int) code = .<fun x -> .~(spower 7 .<x>.)>.
let () = Codelib.print_code Format.std_formatter power7
let power7_native = Runnative.run power7

type t =
  | Null
  | Bool of bool
  | Number of string
  | String of string
  | Array of t list
  | Object of (string * t) list

exception Error of string

val parse : string -> t
val parse_file : string -> t


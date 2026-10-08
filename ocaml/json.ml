type t =
  | Null
  | Bool of bool
  | Number of string
  | String of string
  | Array of t list
  | Object of (string * t) list

exception Error of string

type parser = { source : string; length : int; mutable pos : int }

let fail p message =
  raise (Error (Printf.sprintf "%s at byte %d" message p.pos))

let peek p = if p.pos < p.length then Some p.source.[p.pos] else None
let bump p = p.pos <- p.pos + 1

let rec skip_space p =
  match peek p with
  | Some (' ' | '\n' | '\r' | '\t') -> bump p; skip_space p
  | _ -> ()

let expect_char p expected =
  match peek p with
  | Some actual when actual = expected -> bump p
  | _ -> fail p (Printf.sprintf "expected %C" expected)

let expect_word p word =
  let stop = p.pos + String.length word in
  if stop <= p.length && String.sub p.source p.pos (String.length word) = word
  then p.pos <- stop
  else fail p (Printf.sprintf "expected %s" word)

let hex_value = function
  | '0' .. '9' as c -> Char.code c - Char.code '0'
  | 'a' .. 'f' as c -> 10 + Char.code c - Char.code 'a'
  | 'A' .. 'F' as c -> 10 + Char.code c - Char.code 'A'
  | _ -> -1

let read_hex4 p =
  if p.pos + 4 > p.length then fail p "incomplete unicode escape";
  let value = ref 0 in
  for _ = 1 to 4 do
    let digit = hex_value p.source.[p.pos] in
    if digit < 0 then fail p "invalid unicode escape";
    value := (!value * 16) + digit;
    bump p
  done;
  !value

let add_codepoint p buffer high =
  let codepoint =
    if high >= 0xD800 && high <= 0xDBFF then begin
      if p.pos + 6 > p.length || p.source.[p.pos] <> '\\' || p.source.[p.pos + 1] <> 'u'
      then fail p "high surrogate without low surrogate";
      p.pos <- p.pos + 2;
      let low = read_hex4 p in
      if low < 0xDC00 || low > 0xDFFF then fail p "invalid low surrogate";
      0x10000 + ((high - 0xD800) lsl 10) + (low - 0xDC00)
    end else if high >= 0xDC00 && high <= 0xDFFF then
      fail p "unexpected low surrogate"
    else high
  in
  Buffer.add_utf_8_uchar buffer (Uchar.of_int codepoint)

let parse_string p =
  expect_char p '"';
  let buffer = Buffer.create 64 in
  let rec loop () =
    match peek p with
    | None -> fail p "unterminated string"
    | Some '"' -> bump p; Buffer.contents buffer
    | Some '\\' ->
        bump p;
        begin match peek p with
        | Some '"' -> Buffer.add_char buffer '"'; bump p
        | Some '\\' -> Buffer.add_char buffer '\\'; bump p
        | Some '/' -> Buffer.add_char buffer '/'; bump p
        | Some 'b' -> Buffer.add_char buffer '\b'; bump p
        | Some 'f' -> Buffer.add_char buffer '\012'; bump p
        | Some 'n' -> Buffer.add_char buffer '\n'; bump p
        | Some 'r' -> Buffer.add_char buffer '\r'; bump p
        | Some 't' -> Buffer.add_char buffer '\t'; bump p
        | Some 'u' -> bump p; add_codepoint p buffer (read_hex4 p)
        | _ -> fail p "invalid string escape"
        end;
        loop ()
    | Some c when Char.code c < 0x20 -> fail p "control character in string"
    | Some c -> Buffer.add_char buffer c; bump p; loop ()
  in
  loop ()

let parse_number p =
  let start = p.pos in
  let take predicate =
    while p.pos < p.length && predicate p.source.[p.pos] do bump p done
  in
  begin match peek p with Some '-' -> bump p | _ -> () end;
  begin match peek p with
  | Some '0' -> bump p
  | Some ('1' .. '9') -> take (function '0' .. '9' -> true | _ -> false)
  | _ -> fail p "invalid number"
  end;
  begin match peek p with
  | Some '.' ->
      bump p;
      let fraction_start = p.pos in
      take (function '0' .. '9' -> true | _ -> false);
      if p.pos = fraction_start then fail p "invalid fractional number"
  | _ -> ()
  end;
  begin match peek p with
  | Some ('e' | 'E') ->
      bump p;
      begin match peek p with Some ('+' | '-') -> bump p | _ -> () end;
      let exponent_start = p.pos in
      take (function '0' .. '9' -> true | _ -> false);
      if p.pos = exponent_start then fail p "invalid exponent"
  | _ -> ()
  end;
  Number (String.sub p.source start (p.pos - start))

let rec parse_value p =
  skip_space p;
  match peek p with
  | Some 'n' -> expect_word p "null"; Null
  | Some 't' -> expect_word p "true"; Bool true
  | Some 'f' -> expect_word p "false"; Bool false
  | Some '"' -> String (parse_string p)
  | Some '[' -> parse_array p
  | Some '{' -> parse_object p
  | Some ('-' | '0' .. '9') -> parse_number p
  | Some _ -> fail p "unexpected character"
  | None -> fail p "expected a JSON value"

and parse_array p =
  expect_char p '[';
  skip_space p;
  match peek p with
  | Some ']' -> bump p; Array []
  | _ ->
      let rec entries acc =
        let value = parse_value p in
        skip_space p;
        match peek p with
        | Some ',' -> bump p; skip_space p; entries (value :: acc)
        | Some ']' -> bump p; Array (List.rev (value :: acc))
        | _ -> fail p "expected comma or closing bracket"
      in
      entries []

and parse_object p =
  expect_char p '{';
  skip_space p;
  match peek p with
  | Some '}' -> bump p; Object []
  | _ ->
      let rec fields seen acc =
        skip_space p;
        if peek p <> Some '"' then fail p "object key must be a string";
        let key = parse_string p in
        if List.mem key seen then fail p (Printf.sprintf "duplicate object key %S" key);
        skip_space p;
        expect_char p ':';
        let value = parse_value p in
        skip_space p;
        match peek p with
        | Some ',' -> bump p; fields (key :: seen) ((key, value) :: acc)
        | Some '}' -> bump p; Object (List.rev ((key, value) :: acc))
        | _ -> fail p "expected comma or closing brace"
      in
      fields [] []

let parse source =
  let p = { source; length = String.length source; pos = 0 } in
  let value = parse_value p in
  skip_space p;
  if p.pos <> p.length then fail p "trailing content";
  value

let parse_file path =
  let channel = open_in_bin path in
  Fun.protect
    ~finally:(fun () -> close_in_noerr channel)
    (fun () -> really_input_string channel (in_channel_length channel) |> parse)


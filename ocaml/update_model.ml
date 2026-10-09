open Json

type category = Research | Academia | Writing
type award_outcome = Nominated | Shortlisted | Won
type event_kind =
  | Acceptance
  | Presentation
  | Publication
  | Participation
  | Visit
  | Award of { outcome : award_outcome; name : string }
  | Release
  | Other of string option
type visibility = Draft | Published
type event_status = Planned | Completed
type localized = { en : string; ja : string option }
type link_kind = External | Pdf | Doi | Preprint | Code | Slides | Bibtex | Audio
type link = { label : string; url : string; kind : link_kind }
type related = { publications : string list; projects : string list; writings : string list }
(* The part of the title naming the event, linked to its official page. *)
type title_link = { text : string; url : string; lang : string option }

type update = {
  id : string;
  title : localized;
  title_link : title_link option;
  summary : localized;
  (* The event dates may be left unset only while a record is a draft awaiting
     confirmation; published records always carry a start date. *)
  date : string option;
  end_date : string option;
  event_status : event_status;
  categories : category list;
  kind : event_kind;
  links : link list;
  related : related;
  detail : bool;
  body : localized option;
  visibility : visibility;
}

exception Validation_error of string
let error format = Printf.ksprintf (fun message -> raise (Validation_error message)) format

let as_object field = function Object fields -> fields | _ -> error "%s must be an object" field
let as_array field = function Array values -> values | _ -> error "%s must be an array" field
let as_string field = function String value -> value | _ -> error "%s must be a string" field
let as_bool field = function Bool value -> value | _ -> error "%s must be a boolean" field

let member field fields =
  match List.assoc_opt field fields with Some value -> value | None -> error "missing required field %S" field

let optional field fields = List.assoc_opt field fields

let nonempty field value =
  if String.trim value = "" then error "%s must not be empty" field;
  value

let is_slug value =
  let length = String.length value in
  let rec loop index previous_hyphen =
    if index = length then length > 0 && not previous_hyphen
    else match value.[index] with
      | 'a' .. 'z' | '0' .. '9' -> loop (index + 1) false
      | '-' when index > 0 && not previous_hyphen -> loop (index + 1) true
      | _ -> false
  in
  loop 0 false

let validate_slug field value =
  if not (is_slug value) then error "%s must use lowercase letters, digits, and single hyphens" field;
  value

let is_leap year = year mod 400 = 0 || (year mod 4 = 0 && year mod 100 <> 0)

let validate_date field value =
  let bad () = error "%s must be a real ISO date in YYYY-MM-DD form" field in
  if String.length value <> 10 || value.[4] <> '-' || value.[7] <> '-' then bad ();
  let integer start length =
    try int_of_string (String.sub value start length) with Failure _ -> bad ()
  in
  let year, month, day = integer 0 4, integer 5 2, integer 8 2 in
  let limit = match month with
    | 1 | 3 | 5 | 7 | 8 | 10 | 12 -> 31
    | 4 | 6 | 9 | 11 -> 30
    | 2 -> if is_leap year then 29 else 28
    | _ -> bad ()
  in
  if year < 1 || day < 1 || day > limit then bad ();
  value

let decode_localized field value =
  let fields = as_object field value in
  let en = member "en" fields |> as_string (field ^ ".en") |> nonempty (field ^ ".en") in
  let ja = optional "ja" fields |> Option.map (fun value -> value |> as_string (field ^ ".ja") |> nonempty (field ^ ".ja")) in
  { en; ja }

let decode_category = function
  | String "research" -> Research
  | String "academia" -> Academia
  | String "writing" -> Writing
  | String value -> error "invalid category %S (expected research, academia, or writing)" value
  | _ -> error "each category must be a string"

let decode_categories value =
  let categories = as_array "categories" value |> List.map decode_category in
  if categories = [] then error "categories must contain at least one value";
  let rec unique = function
    | [] -> ()
    | head :: tail -> if List.mem head tail then error "categories must not contain duplicates" else unique tail
  in
  unique categories;
  categories

let decode_kind value =
  let fields = as_object "kind" value in
  match member "type" fields |> as_string "kind.type" with
  | "acceptance" -> Acceptance
  | "presentation" -> Presentation
  | "publication" -> Publication
  | "participation" -> Participation
  | "visit" -> Visit
  | "release" -> Release
  | "other" -> Other (optional "detail" fields |> Option.map (as_string "kind.detail"))
  | "award" ->
      let outcome = match member "outcome" fields |> as_string "kind.outcome" with
        | "nominated" -> Nominated
        | "shortlisted" -> Shortlisted
        | "won" -> Won
        | value -> error "invalid award outcome %S (expected nominated, shortlisted, or won)" value
      in
      let name = member "name" fields |> as_string "kind.name" |> nonempty "kind.name" in
      Award { outcome; name }
  | value -> error "invalid event kind %S" value

let decode_link_kind = function
  | "external" -> External | "pdf" -> Pdf | "doi" -> Doi | "preprint" -> Preprint
  | "code" -> Code | "slides" -> Slides | "bibtex" -> Bibtex | "audio" -> Audio
  | value -> error "invalid link type %S" value

let valid_url value =
  let starts prefix = String.starts_with ~prefix value in
  starts "https://" || starts "http://" || starts "/" || starts "mailto:"

let decode_link value =
  let fields = as_object "link" value in
  let label = member "label" fields |> as_string "link.label" |> nonempty "link.label" in
  let url = member "url" fields |> as_string "link.url" |> nonempty "link.url" in
  if not (valid_url url) then error "link.url must begin with https://, http://, /, or mailto:";
  let kind = match optional "type" fields with None -> External | Some value -> value |> as_string "link.type" |> decode_link_kind in
  { label; url; kind }

let occurrences needle haystack =
  let n = String.length needle and h = String.length haystack in
  let rec loop index count =
    if index + n > h then count
    else loop (index + 1) (if String.sub haystack index n = needle then count + 1 else count)
  in
  loop 0 0

let decode_title_link (title : localized) value =
  let fields = as_object "title_link" value in
  let text = member "text" fields |> as_string "title_link.text" |> nonempty "title_link.text" in
  if occurrences text title.en <> 1 then error "title_link.text must occur exactly once in title.en";
  let url = member "url" fields |> as_string "title_link.url" |> nonempty "title_link.url" in
  if not (String.starts_with ~prefix:"https://" url || String.starts_with ~prefix:"http://" url) then
    error "title_link.url must begin with https:// or http://";
  let lang = optional "lang" fields |> Option.map (fun value ->
    match as_string "title_link.lang" value with
    | ("en" | "ja") as lang -> lang
    | lang -> error "invalid title_link.lang %S (expected en or ja)" lang) in
  { text; url; lang }

let decode_identifier_list field fields =
  match optional field fields with
  | None -> []
  | Some value -> as_array ("related." ^ field) value |> List.map (fun item -> item |> as_string ("related." ^ field) |> validate_slug ("related." ^ field))

let decode_related = function
  | None -> { publications = []; projects = []; writings = [] }
  | Some value ->
      let fields = as_object "related" value in
      { publications = decode_identifier_list "publications" fields;
        projects = decode_identifier_list "projects" fields;
        writings = decode_identifier_list "writings" fields }

let decode_visibility = function
  | String "draft" -> Draft
  | String "published" -> Published
  | String value -> error "invalid status %S (expected draft or published)" value
  | _ -> error "status must be a string"

let decode_event_status = function
  | String "planned" -> Planned
  | String "completed" -> Completed
  | String value -> error "invalid event_status %S (expected planned or completed)" value
  | _ -> error "event_status must be a string"

(* ISO dates compare chronologically as strings once validated. *)
let validate_event_dates ~date ~end_date =
  let last_day = Option.value end_date ~default:date in
  if String.compare last_day date < 0 then error "end_date must not precede date"

let decode value =
  let fields = as_object "update" value in
  let id = member "id" fields |> as_string "id" |> validate_slug "id" in
  let title = member "title" fields |> decode_localized "title" in
  let title_link = optional "title_link" fields |> Option.map (decode_title_link title) in
  let summary = member "summary" fields |> decode_localized "summary" in
  let optional_date field = optional field fields |> Option.map (fun value -> value |> as_string field |> validate_date field) in
  let date = optional_date "date" in
  let end_date = optional_date "end_date" in
  let event_status = member "event_status" fields |> decode_event_status in
  let visibility = member "status" fields |> decode_visibility in
  (match date with
   | Some date -> validate_event_dates ~date ~end_date
   | None when visibility = Published -> error "a published update requires date"
   | None when end_date <> None -> error "end_date requires date"
   | None -> ());
  let categories = member "categories" fields |> decode_categories in
  let kind = member "kind" fields |> decode_kind in
  let links = match optional "links" fields with None -> [] | Some value -> as_array "links" value |> List.map decode_link in
  let related = decode_related (optional "related" fields) in
  let detail = match optional "detail" fields with None -> false | Some value -> as_bool "detail" value in
  let body = optional "body" fields |> Option.map (decode_localized "body") in
  { id; title; title_link; summary; date; end_date; event_status; categories; kind; links; related; detail; body; visibility }

let id update = update.id
let is_published update = update.visibility = Published

let required_date field = function
  | Some value -> value
  | None -> error "%s is required to export an update" field

let escape value =
  let buffer = Buffer.create (String.length value + 8) in
  String.iter (function
    | '"' -> Buffer.add_string buffer "\\\""
    | '\\' -> Buffer.add_string buffer "\\\\"
    | '\n' -> Buffer.add_string buffer "\\n"
    | '\r' -> Buffer.add_string buffer "\\r"
    | '\t' -> Buffer.add_string buffer "\\t"
    | c when Char.code c < 0x20 -> Buffer.add_string buffer (Printf.sprintf "\\u%04x" (Char.code c))
    | c -> Buffer.add_char buffer c
  ) value;
  Buffer.contents buffer

let quote value = Printf.sprintf "\"%s\"" (escape value)
let option_field key = function None -> "" | Some value -> Printf.sprintf ",\"%s\":%s" key (quote value)

let localized_json value = Printf.sprintf "{\"en\":%s%s}" (quote value.en) (option_field "ja" value.ja)

let event_status_name = function Planned -> "planned" | Completed -> "completed"
let category_name = function Research -> "research" | Academia -> "academia" | Writing -> "writing"
let outcome_name = function Nominated -> "nominated" | Shortlisted -> "shortlisted" | Won -> "won"
let kind_json = function
  | Acceptance -> "{\"type\":\"acceptance\"}"
  | Presentation -> "{\"type\":\"presentation\"}"
  | Publication -> "{\"type\":\"publication\"}"
  | Participation -> "{\"type\":\"participation\"}"
  | Visit -> "{\"type\":\"visit\"}"
  | Release -> "{\"type\":\"release\"}"
  | Other detail -> Printf.sprintf "{\"type\":\"other\"%s}" (option_field "detail" detail)
  | Award { outcome; name } -> Printf.sprintf "{\"type\":\"award\",\"outcome\":%s,\"name\":%s}" (quote (outcome_name outcome)) (quote name)

let link_kind_name = function
  | External -> "external" | Pdf -> "pdf" | Doi -> "doi" | Preprint -> "preprint"
  | Code -> "code" | Slides -> "slides" | Bibtex -> "bibtex" | Audio -> "audio"

let link_json link = Printf.sprintf "{\"label\":%s,\"url\":%s,\"type\":%s}" (quote link.label) (quote link.url) (quote (link_kind_name link.kind))
let title_link_json = function
  | None -> ""
  | Some link -> Printf.sprintf ",\"titleLink\":{\"text\":%s,\"url\":%s%s}" (quote link.text) (quote link.url) (option_field "lang" link.lang)
let array render values = "[" ^ String.concat "," (List.map render values) ^ "]"
let string_array values = array quote values

let to_json update =
  let body = match update.body with None -> "" | Some value -> ",\"body\":" ^ localized_json value in
  Printf.sprintf
    "{\"id\":%s,\"title\":%s%s,\"summary\":%s,\"date\":%s%s,\"eventStatus\":%s,\"categories\":%s,\"kind\":%s,\"links\":%s,\"related\":{\"publications\":%s,\"projects\":%s,\"writings\":%s},\"detail\":%s%s}"
    (quote update.id)
    (localized_json update.title)
    (title_link_json update.title_link)
    (localized_json update.summary)
    (quote (required_date "date" update.date))
    (option_field "endDate" update.end_date)
    (quote (event_status_name update.event_status))
    (array (fun category -> quote (category_name category)) update.categories)
    (kind_json update.kind)
    (array link_json update.links)
    (string_array update.related.publications)
    (string_array update.related.projects)
    (string_array update.related.writings)
    (string_of_bool update.detail)
    body

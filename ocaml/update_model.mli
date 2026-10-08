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

type update

exception Validation_error of string

val decode : Json.t -> update
val id : update -> string
val is_published : update -> bool
val announced_on : update -> string
val to_json : update -> string


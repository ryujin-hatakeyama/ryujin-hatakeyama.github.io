open Content_pipeline

let good_record = {|{
  "id":"test-record",
  "title":{"en":"Test record","ja":"テスト記録"},
  "summary":{"en":"A private validation fixture."},
  "date":"2024-02-29",
  "announced_on":"2024-03-01",
  "event_status":"completed",
  "categories":["research","writing"],
  "kind":{"type":"award","outcome":"shortlisted","name":"Fixture award"},
  "links":[],
  "status":"draft"
}|}

let expect_invalid label source =
  try
    ignore (Json.parse source |> Update_model.decode);
    failwith (label ^ " unexpectedly passed validation")
  with Update_model.Validation_error _ -> ()

let () =
  let record = Json.parse good_record |> Update_model.decode in
  assert (Update_model.id record = "test-record");
  assert (not (Update_model.is_published record));
  expect_invalid "invalid date" (String.concat "" [
    {|{"id":"bad-date","title":{"en":"x"},"summary":{"en":"x"},"date":"2023-02-29",|};
    {|"announced_on":"2024-01-01","event_status":"completed","categories":["research"],"kind":{"type":"other"},"status":"draft"}|}
  ]);
  expect_invalid "invalid category" (String.concat "" [
    {|{"id":"bad-category","title":{"en":"x"},"summary":{"en":"x"},"date":"2024-01-01",|};
    {|"announced_on":"2024-01-01","event_status":"completed","categories":["personal"],"kind":{"type":"other"},"status":"draft"}|}
  ]);
  expect_invalid "ambiguous award" (String.concat "" [
    {|{"id":"bad-award","title":{"en":"x"},"summary":{"en":"x"},"date":"2024-01-01",|};
    {|"announced_on":"2024-01-01","event_status":"completed","categories":["writing"],"kind":{"type":"award","name":"Fixture"},"status":"draft"}|}
  ]);
  let event fields = String.concat "" [
    {|{"id":"event","title":{"en":"x"},"summary":{"en":"x"},"categories":["academia"],|};
    {|"kind":{"type":"participation"},"status":"published",|}; fields; "}"
  ] in
  let planned = Json.parse (event {|"date":"2026-10-16","end_date":"2026-10-18","announced_on":"2026-10-09","event_status":"planned"|})
    |> Update_model.decode |> Update_model.to_json in
  let contains needle =
    let n = String.length needle and h = String.length planned in
    let rec loop i = i + n <= h && (String.sub planned i n = needle || loop (i + 1)) in
    loop 0
  in
  assert (contains {|"endDate":"2026-10-18"|});
  assert (contains {|"eventStatus":"planned"|});
  expect_invalid "missing event status" (event {|"date":"2026-10-16","announced_on":"2026-10-09"|});
  expect_invalid "unknown event status" (event {|"date":"2026-10-16","announced_on":"2026-10-09","event_status":"attended"|});
  expect_invalid "end before start" (event {|"date":"2026-10-18","end_date":"2026-10-16","announced_on":"2026-10-19","event_status":"completed"|});
  expect_invalid "planned after start" (event {|"date":"2026-10-16","end_date":"2026-10-18","announced_on":"2026-10-16","event_status":"planned"|});
  expect_invalid "completed before end" (event {|"date":"2026-10-16","end_date":"2026-10-18","announced_on":"2026-10-17","event_status":"completed"|});
  let undated status = String.concat "" [
    {|{"id":"undated","title":{"en":"x"},"summary":{"en":"x"},"categories":["academia"],|};
    {|"kind":{"type":"participation"},"event_status":"completed","status":"|}; status; {|"}|}
  ] in
  let draft = Json.parse (undated "draft") |> Update_model.decode in
  assert (not (Update_model.is_published draft));
  assert (Update_model.announced_on draft = None);
  expect_invalid "published without dates" (undated "published");
  print_endline "OCaml update model tests passed."


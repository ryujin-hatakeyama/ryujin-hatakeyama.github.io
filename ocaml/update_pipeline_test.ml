open Content_pipeline

let good_record = {|{
  "id":"test-record",
  "title":{"en":"Test record","ja":"テスト記録"},
  "summary":{"en":"A private validation fixture."},
  "date":"2024-02-29",
  "announced_on":"2024-03-01",
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
    {|"announced_on":"2024-01-01","categories":["research"],"kind":{"type":"other"},"status":"draft"}|}
  ]);
  expect_invalid "invalid category" (String.concat "" [
    {|{"id":"bad-category","title":{"en":"x"},"summary":{"en":"x"},"date":"2024-01-01",|};
    {|"announced_on":"2024-01-01","categories":["personal"],"kind":{"type":"other"},"status":"draft"}|}
  ]);
  expect_invalid "ambiguous award" (String.concat "" [
    {|{"id":"bad-award","title":{"en":"x"},"summary":{"en":"x"},"date":"2024-01-01",|};
    {|"announced_on":"2024-01-01","categories":["writing"],"kind":{"type":"award","name":"Fixture"},"status":"draft"}|}
  ]);
  print_endline "OCaml update model tests passed."


open Content_pipeline

let has_json_suffix name = Filename.check_suffix name ".json"

let read_update directory name =
  let path = Filename.concat directory name in
  try Json.parse_file path |> Update_model.decode with
  | Json.Error message -> failwith (Printf.sprintf "%s: invalid JSON: %s" path message)
  | Update_model.Validation_error message -> failwith (Printf.sprintf "%s: %s" path message)

let reject_duplicate_ids updates =
  let seen = Hashtbl.create (List.length updates) in
  List.iter (fun update ->
    let identifier = Update_model.id update in
    match Hashtbl.find_opt seen identifier with
    | Some _ -> failwith (Printf.sprintf "duplicate update id %S" identifier)
    | None -> Hashtbl.add seen identifier ()
  ) updates

let write_atomically path contents =
  let temporary = path ^ ".tmp" in
  let channel = open_out_bin temporary in
  try
    output_string channel contents;
    output_char channel '\n';
    close_out channel;
    Sys.rename temporary path
  with failure ->
    close_out_noerr channel;
    (try Sys.remove temporary with Sys_error _ -> ());
    raise failure

let () =
  if Array.length Sys.argv <> 3 then begin
    prerr_endline "usage: generate_updates <content-directory> <output.json>";
    exit 2
  end;
  let directory, output = Sys.argv.(1), Sys.argv.(2) in
  try
    let updates =
      Sys.readdir directory
      |> Array.to_list
      |> List.filter has_json_suffix
      |> List.sort String.compare
      |> List.map (read_update directory)
    in
    reject_duplicate_ids updates;
    (* Exported in file-name order; the site builder orders records by event date. *)
    let public_updates = List.filter Update_model.is_published updates in
    let json = "[" ^ String.concat "," (List.map Update_model.to_json public_updates) ^ "]" in
    write_atomically output json;
    Printf.printf "Validated %d update record(s); exported %d published record(s).\n"
      (List.length updates) (List.length public_updates)
  with
  | Failure message -> prerr_endline ("Update generation failed: " ^ message); exit 1
  | Sys_error message -> prerr_endline ("Update generation failed: " ^ message); exit 1

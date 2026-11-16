use serde_json::{Map, Value, json};

pub fn document() -> Value {
    let kinds = crate::puzzles::PuzzleKind::ALL.map(|kind| kind.slug());
    let content_version = json!({"type":"string","enum":["1","2"],"description":"Language puzzles only. Omit for latest (2). Send data.content_version to replay or verify an older puzzle."});
    let generation = json!({"type":"object","additionalProperties":false,"required":["kind"],"properties":{
        "kind":{"type":"string","enum":kinds},
        "seed":{"type":"integer","minimum":0,"maximum":18446744073709551615_u64,"description":"Unsigned 64-bit seed. Omit to choose a fresh seed. Use a bigint-aware JSON client for exact integers beyond 2^53-1."},
        "difficulty":{"type":"string","enum":["easy","medium","hard"],"default":"medium","description":"Generation preset, not a calibrated human difficulty rating."},
        "include_solution":{"type":"boolean","default":false}, "content_version":content_version
    }});
    let mut paths = Map::new();
    paths.insert("/api/v1/catalog".into(), json!({"get":{"summary":"List generators, rules, answer formats, and uniqueness guarantees","security":[],"responses":{"200":{"description":"Generator catalog","content":{"application/json":{"schema":{"type":"object","properties":{"kinds":{"type":"array","items":{"type":"object"}}}}}}}}}}));
    let mut operation = compute(
        "Generate one puzzle",
        json!({"$ref":"#/components/schemas/Puzzle"}),
    );
    operation["requestBody"] = body(json!({"$ref":"#/components/schemas/GenerateRequest"}));
    paths.insert("/api/v1/generate".into(), json!({"post":operation}));
    let mut batch = compute(
        "Generate 1 to 16 puzzles with bounded CPU work",
        json!({"type":"object","properties":{"puzzles":{"type":"array","items":{"$ref":"#/components/schemas/Puzzle"}}}}),
    );
    batch["requestBody"] = body(
        json!({"type":"object","additionalProperties":false,"required":["puzzles"],"properties":{"puzzles":{"type":"array","minItems":1,"maxItems":16,"items":{"$ref":"#/components/schemas/GenerateRequest"}}}}),
    );
    paths.insert("/api/v1/batch".into(), json!({"post":batch}));
    let mut verify = compute(
        "Verify an answer by regenerating its puzzle",
        json!({"type":"object","required":["correct"],"properties":{"correct":{"type":"boolean"}}}),
    );
    verify["requestBody"] = body(schema_ref("VerifyRequest"));
    paths.insert("/api/v1/verify".into(), json!({"post":verify}));
    let params = json!([
        {"name":"kind","in":"path","required":true,"schema":{"type":"string","enum":kinds}},
        {"name":"seed","in":"query","schema":generation["properties"]["seed"]},
        {"name":"difficulty","in":"query","schema":generation["properties"]["difficulty"]},
        {"name":"include_solution","in":"query","schema":{"type":"boolean","default":false}},
        {"name":"content_version","in":"query","schema":content_version}
    ]);
    let mut get = compute(
        "Generate a puzzle using URL parameters",
        json!({"$ref":"#/components/schemas/Puzzle"}),
    );
    get["parameters"] = params;
    paths.insert("/api/v1/puzzles/{kind}".into(), json!({"get":get}));
    let mut daily = compute(
        "Get a deterministic daily puzzle using a UTC calendar date",
        json!({"type":"object","properties":{"date":{"type":"string","format":"date"},"timezone":{"const":"UTC"},"puzzle":{"$ref":"#/components/schemas/Puzzle"}}}),
    );
    daily["parameters"] = json!([
        {"name":"kind","in":"path","required":true,"schema":{"type":"string","enum":kinds}},
        {"name":"date","in":"query","schema":{"type":"string","format":"date"},"description":"Defaults to the current UTC date."},
        {"name":"difficulty","in":"query","schema":generation["properties"]["difficulty"]},
        {"name":"include_solution","in":"query","schema":{"type":"boolean","default":false}},
        {"name":"content_version","in":"query","schema":content_version}
    ]);
    paths.insert("/api/v1/daily/{kind}".into(), json!({"get":daily}));
    for (path, summary) in [
        ("/health/live", "Liveness"),
        (
            "/health/ready",
            "Stateless API readiness; frontend assets are independent",
        ),
    ] {
        paths.insert(path.into(), json!({"get":{"summary":summary,"security":[],"responses":{"200":{"description":"Healthy","content":{"application/json":{"schema":{"type":"object"}}}}}}}));
    }
    paths.insert("/metrics".into(), json!({"get":{"summary":"Prometheus instance counters; requires a key when configured","responses":{"200":{"description":"Metrics","content":{"text/plain":{"schema":{"type":"string"}}}},"401":{"$ref":"#/components/responses/Error"}}}}));
    paths.insert("/api/v1/openapi.json".into(), json!({"get":{"summary":"This OpenAPI 3.1 document","security":[],"responses":{"200":{"description":"OpenAPI document","content":{"application/json":{"schema":{"type":"object"}}}}}}}));
    let mut document = json!({"openapi":"3.1.0","info":{"title":"Puzzgen API","version":"0.2.0","description":"Procedural puzzles and member accounts for learning. With DATABASE_URL, computation requires a member key or same-origin session and consumes shared account quotas. Without it, legacy public/static-key mode is available. Catalog and learning resources remain public. Exposed seeds and solutions are construction tools, not a competitive anti-cheat boundary."},
        "servers":[{"url":"/"}],"security":[{"bearerAuth":[]}],"paths":paths,
        "components":{"securitySchemes":{"bearerAuth":{"type":"http","scheme":"bearer","description":"Optional static API key configured through PUZZGEN_API_KEYS; not an OAuth access token."}},
            "schemas":typed_schemas(generation),
            "responses":{"Error":{"description":"JSON error with machine-readable code and human-readable message","content":{"application/json":{"schema":{"$ref":"#/components/schemas/Error"}}}}}
        }
    });
    crate::accounts::extend_openapi(&mut document);
    document
}

/// Schemas describe JSON shapes. Gameplay validation additionally checks clues,
/// legal moves, coordinates and dimensions against the original seeded board.
fn typed_schemas(mut generation: Value) -> Map<String, Value> {
    let mut schemas = Map::new();
    let seed = generation["properties"]["seed"].clone();
    let difficulty = generation["properties"]["difficulty"].clone();
    generation["properties"]["seed"] = json!({"anyOf":[seed.clone(),{"type":"null"}],"description":"Omit or send null for a fresh seed; otherwise send an unsigned 64-bit integer."});
    schemas.insert("GenerateRequest".into(), generation);
    schemas.insert("UnsignedSeed".into(), seed);
    schemas.insert("Difficulty".into(), difficulty);
    schemas.insert("SeedString".into(), json!({"type":"string","pattern":"^[0-9]+$","minLength":1,"maxLength":20,"description":"Decimal unsigned 64-bit seed string preserves every bit in JavaScript. Convert exactly to an integer for POST requests or pass unchanged in GET query parameters."}));
    schemas.insert(
        "Coordinate".into(),
        described(
            array(integer(0, 24), 2, 2),
            "Zero-based [row,column]. Coordinates must also lie inside the selected puzzle board.",
        ),
    );
    schemas.insert("BinaryCell".into(), integer(0, 1));

    schemas.insert("SudokuData".into(), object(json!({"grid":grid(integer(0,9),&[9]),"box_size":{"type":"integer","const":3},"clue_count":integer(28,81)}), &["grid","box_size","clue_count"]));
    schemas.insert("MiniSudokuData".into(), object(json!({"grid":grid(integer(0,4),&[4]),"box_size":{"type":"integer","const":2},"clue_count":integer(6,16)}), &["grid","box_size","clue_count"]));
    schemas.insert(
        "SudokuAnswer".into(),
        described(
            grid(integer(1, 9), &[9]),
            "Completed 9x9 grid preserving givens and satisfying every row, column, and 3x3 box.",
        ),
    );
    schemas.insert(
        "MiniSudokuAnswer".into(),
        described(
            grid(integer(1, 4), &[4]),
            "Completed 4x4 grid preserving givens and satisfying every row, column, and 2x2 box.",
        ),
    );

    schemas.insert("MazeData".into(), object(json!({"grid":described(grid(schema_ref("BinaryCell"),&[11,17,25]),"0 is open and 1 is a wall."),"start":schema_ref("Coordinate"),"end":schema_ref("Coordinate")}), &["grid","start","end"]));
    schemas.insert("MazeAnswer".into(), described(array(schema_ref("Coordinate"),2,2500),"Orthogonally adjacent open cells including start and end. Repeated cells are allowed; length is limited to four times this board's area."));

    let line_clues = described(
        array(integer(1, 9), 0, 5),
        "Lengths of consecutive filled runs. An empty array means an empty line.",
    );
    schemas.insert("NonogramData".into(), object(json!({"rows":{"type":"integer","enum":[5,7,9]},"cols":{"type":"integer","enum":[5,7,9]},"row_clues":array(line_clues.clone(),5,9),"column_clues":array(line_clues,5,9)}), &["rows","cols","row_clues","column_clues"]));
    schemas.insert("NonogramAnswer".into(), described(grid(schema_ref("BinaryCell"),&[5,7,9]),"Binary filled-cell grid matching every run clue. Dimensions must match rows and cols. Generated boards are uniquely solvable."));

    schemas.insert("WordSearchData".into(), object(json!({"grid":grid(json!({"type":"string","pattern":"^[A-Z]$"}),&[10,12,15]),"words":array(json!({"type":"string","pattern":"^[A-Z]+$"}),0,10)}), &["grid","words"]));
    let mut placement = object(
        json!({"word":{"type":"string","pattern":"^[A-Z]+$"},"cells":array(schema_ref("Coordinate"),2,15)}),
        &["word", "cells"],
    );
    placement["additionalProperties"] = json!(false);
    placement["description"] = json!(
        "Cells must spell this word in a straight horizontal, vertical or diagonal line, in order."
    );
    schemas.insert("WordPlacement".into(), placement);
    schemas.insert("WordSearchAnswer".into(), described(array(schema_ref("WordPlacement"),0,10),"One placement per listed word, in any order. Alternate occurrences are accepted; missing or duplicate words are rejected."));

    schemas.insert("SlidingPuzzleData".into(), object(json!({"grid":{"oneOf":[grid(integer(0,8),&[3]),grid(integer(0,15),&[4])]},"size":{"type":"integer","enum":[3,4]}}), &["grid","size"]));
    let mut sliding = object(
        json!({"moves":array(json!({"type":"string","enum":["up","down","left","right"]}),0,10_000)}),
        &["moves"],
    );
    sliding["additionalProperties"] = json!(false);
    sliding["description"] = json!(
        "Legal movements of the EMPTY cell, ending in row-major order with zero last. The move sequence need not be shortest."
    );
    schemas.insert("SlidingPuzzleAnswer".into(), sliding);

    schemas.insert("LightsOutData".into(), object(json!({"grid":described(grid(schema_ref("BinaryCell"),&[4,5,6]),"0 is off and 1 is on.")}), &["grid"]));
    schemas.insert("LightsOutAnswer".into(), described(array(schema_ref("Coordinate"),0,144),"Press coordinates in any order. Each toggles itself and orthogonal neighbors; repeated presses are allowed. At most four times this board's area may be submitted. All lights must end off."));

    schemas.insert(
        "RevealedMineCell".into(),
        object(
            json!({"row":integer(0,15),"col":integer(0,15),"adjacent":integer(0,8)}),
            &["row", "col", "adjacent"],
        ),
    );
    schemas.insert("MinesweeperData".into(), object(json!({"rows":{"type":"integer","enum":[8,12,16]},"cols":{"type":"integer","enum":[8,12,16]},"mine_count":{"type":"integer","enum":[10,24,50]},"start":schema_ref("Coordinate"),"initial_revealed":array(schema_ref("RevealedMineCell"),1,206)}), &["rows","cols","mine_count","start","initial_revealed"]));
    let mut mines = described(
        array(schema_ref("Coordinate"), 10, 50),
        "All mine coordinates in any order, with no duplicates. Count must equal mine_count. Safe-start generation does not guarantee no-guess solvability.",
    );
    mines["uniqueItems"] = json!(true);
    schemas.insert("MinesweeperAnswer".into(), mines);

    schemas.insert("ArithmeticData".into(), object(json!({"expression":{"type":"string","description":"Integer expression with +, -, *, and parentheses; normal arithmetic precedence applies."}}), &["expression"]));
    schemas.insert("ArithmeticAnswer".into(), json!({"type":"integer","format":"int64","description":"Exact integer result of the generated expression."}));
    schemas.insert("CryptogramData".into(), object(json!({"ciphertext":{"type":"string","description":"Uppercase monoalphabetic substitution ciphertext, preserving spaces and punctuation."}}), &["ciphertext"]));
    schemas.insert("CryptogramAnswer".into(), json!({"type":"string","description":"Selected plaintext phrase. ASCII case and repeated, leading, or trailing whitespace are ignored. Linguistic uniqueness is not guaranteed."}));

    let word = json!({"type":"string","pattern":"^[a-z]+$"});
    let phrase = json!({"type":"string"});
    for (title, properties, required) in [
        (
            "Spoonerism",
            json!({"source_phrase":phrase,"clue":phrase,"word_lengths":array(integer(1,32),2,2),"choices":array(phrase.clone(),4,4),"hint":phrase,"source_sounds":array(phrase.clone(),2,2)}),
            vec!["source_phrase", "clue", "word_lengths"],
        ),
        (
            "Anagram",
            json!({"letters":word,"length":integer(3,32),"clue":phrase,"first_letter":word,"allowed_words":array(word.clone(),1,1000)}),
            vec!["letters", "length"],
        ),
        (
            "WordLadder",
            json!({"start":word,"end":word,"allowed_words":array(word.clone(),1,1000),"max_steps":integer(1,32),"minimum_steps":integer(1,32)}),
            vec![
                "start",
                "end",
                "allowed_words",
                "max_steps",
                "minimum_steps",
            ],
        ),
        (
            "CompoundBridge",
            json!({"left":word,"right":word,"length":integer(1,32),"clue":phrase,"first_letter":word}),
            vec!["left", "right", "length"],
        ),
    ] {
        let mut data = object(properties, &required);
        data["properties"]["content_version"] = json!({"type":"string","enum":["1","2"]});
        data["properties"]["learning"] = object(
            json!({"objective":{"type":"string"},"principle":{"type":"string"},"experiment":{"type":"string"}}),
            &["objective", "principle", "experiment"],
        );
        data["properties"]["locale"] = json!({"type":"string","const":"en-US"});
        data["properties"]["content_license"] = json!({"type":"string","const":"CC0-1.0"});
        data["required"].as_array_mut().unwrap().extend([
            json!("content_version"),
            json!("locale"),
            json!("content_license"),
        ]);
        schemas.insert(format!("{title}Data"), data);
        schemas.insert(format!("{title}Answer"),if title=="WordLadder" {described(array(phrase.clone(),2,33),"Any legal path within max_steps, including endpoints, using allowed_words without repeats. Case and whitespace normalized.")} else {described(phrase.clone(),"Curated answer; ASCII case and whitespace normalized. Hard anagrams accept all allowed_words with exactly the supplied letters.")});
    }
    let id = json!({"type":"string","pattern":"^[a-z][0-9]+$"});
    schemas.insert("WordSortingData".into(),object(json!({"categories":array(object(json!({"id":id,"label":{"type":"string"}}),&["id","label"]),2,4),"items":array(object(json!({"id":id,"text":{"type":"string"}}),&["id","text"]),6,12)}),&["categories","items"]));
    schemas.insert("WordSortingAnswer".into(),json!({"type":"object","minProperties":6,"maxProperties":12,"additionalProperties":id,"description":"Exact mapping of every item ID to one category ID; no extra keys."}));
    schemas.insert("SequenceDetectiveData".into(),object(json!({"terms":array(json!({"type":["integer","null"]}),6,8),"missing_index":integer(1,6),"family":{"type":"string","enum":["constant addition","constant multiplication","consecutive positive integer squares"]}}),&["terms","missing_index","family"]));
    schemas.insert("SequenceDetectiveAnswer".into(),json!({"type":"integer","description":"Missing integer within the named family, not an unrestricted inferred rule."}));
    schemas.insert("MemoryPairsData".into(),object(json!({"cards":array(object(json!({"id":id,"face":{"type":"string"}}),&["id","face"]),6,14),"pair_count":integer(3,7)}),&["cards","pair_count"]));
    schemas.insert("MemoryPairsAnswer".into(),described(array(array(id.clone(),2,2),3,7),"Every card exactly once in equal-face pairs. Pair order and card order inside pairs are ignored."));
    schemas.insert("PathConnectionsData".into(),object(json!({"size":integer(3,5),"grid":grid(integer(1,15),&[3,4,5]),"source":schema_ref("Coordinate"),"goal":schema_ref("Coordinate"),"direction_bits":object(json!({"north":{"const":1},"east":{"const":2},"south":{"const":4},"west":{"const":8}}),&["north","east","south","west"])}),&["size","grid","source","goal","direction_bits"]));
    schemas.insert("PathConnectionsAnswer".into(),described(grid(integer(1,15),&[3,4,5]),"Rotate each original port mask. All ports must meet opposite ports, without boundary leaks, and all tiles must connect to the source."));
    schemas.insert("FutoshikiData".into(),object(json!({"size":{"const":4},"grid":grid(integer(0,4),&[4]),"clue_count":integer(4,16),"inequalities":array(object(json!({"first":schema_ref("Coordinate"),"second":schema_ref("Coordinate"),"op":{"type":"string","enum":["<",">"]}}),&["first","second","op"]),6,10)}),&["size","grid","clue_count","inequalities"]));
    schemas.insert(
        "FutoshikiAnswer".into(),
        described(
            grid(integer(1, 4), &[4]),
            "Latin square satisfying every given and inequality; boxes are not constrained.",
        ),
    );
    schemas.insert("SoundFamiliesData".into(),object(json!({"anchor":{"type":"string"},"words":array(object(json!({"id":id,"text":{"type":"string"}}),&["id","text"]),5,7),"convention":{"type":"string"},"hint":{"type":"string"}}),&["anchor","words","convention","hint"]));
    schemas.insert(
        "SoundFamiliesAnswer".into(),
        described(
            array(id.clone(), 3, 3),
            "All three rhyming candidate IDs, in any order, without duplicates.",
        ),
    );
    schemas.insert("SentenceAssemblyData".into(),object(json!({"tiles":array(object(json!({"id":id,"text":{"type":"string"}}),&["id","text"]),5,5),"hint":{"type":"string"}}),&["tiles","hint"]));
    schemas.insert("SentenceAssemblyAnswer".into(),described(array(id.clone(),5,5),"Every tile ID once. Subject is the adjective animal. Adverb may be initial, before the verb, or final."));
    let mut puzzle_variants = Vec::new();
    let mut verify_variants = Vec::new();
    let mut puzzle_mapping = Map::new();
    let mut verify_mapping = Map::new();
    for (kind, title) in [
        ("sudoku", "Sudoku"),
        ("mini-sudoku", "MiniSudoku"),
        ("maze", "Maze"),
        ("nonogram", "Nonogram"),
        ("word-search", "WordSearch"),
        ("sliding-puzzle", "SlidingPuzzle"),
        ("lights-out", "LightsOut"),
        ("minesweeper", "Minesweeper"),
        ("arithmetic", "Arithmetic"),
        ("cryptogram", "Cryptogram"),
        ("spoonerism", "Spoonerism"),
        ("anagram", "Anagram"),
        ("word-ladder", "WordLadder"),
        ("compound-bridge", "CompoundBridge"),
        ("word-sorting", "WordSorting"),
        ("sequence-detective", "SequenceDetective"),
        ("memory-pairs", "MemoryPairs"),
        ("path-connections", "PathConnections"),
        ("futoshiki", "Futoshiki"),
        ("sound-families", "SoundFamilies"),
        ("sentence-assembly", "SentenceAssembly"),
    ] {
        let puzzle_name = format!("{title}Puzzle");
        let verify_name = format!("Verify{title}Request");
        let answer_name = format!("{title}Answer");
        let data_name = format!("{title}Data");
        let puzzle = object(
            json!({
                "id":{"type":"string"},"generator_version":{"type":"string"},"kind":{"type":"string","const":kind},
                "seed":schema_ref("SeedString"),"difficulty":schema_ref("Difficulty"),"data":schema_ref(&data_name),
                "solution":described(schema_ref(&answer_name),"Present only when include_solution is true."),
                "rules":{"type":"string"},"uniqueness":{"type":"string"}
            }),
            &[
                "id",
                "generator_version",
                "kind",
                "seed",
                "difficulty",
                "data",
                "rules",
                "uniqueness",
            ],
        );
        let mut verify = object(
            json!({"kind":{"type":"string","const":kind},"seed":schema_ref("UnsignedSeed"),"difficulty":schema_ref("Difficulty"),"answer":schema_ref(&answer_name)}),
            &["kind", "seed", "answer"],
        );
        if ["spoonerism", "anagram", "word-ladder", "compound-bridge"].contains(&kind) {
            verify["properties"]["content_version"] =
                schemas["GenerateRequest"]["properties"]["content_version"].clone();
        }
        verify["additionalProperties"] = json!(false);
        schemas.insert(puzzle_name.clone(), puzzle);
        schemas.insert(verify_name.clone(), verify);
        puzzle_variants.push(schema_ref(&puzzle_name));
        verify_variants.push(schema_ref(&verify_name));
        puzzle_mapping.insert(
            kind.into(),
            json!(format!("#/components/schemas/{puzzle_name}")),
        );
        verify_mapping.insert(
            kind.into(),
            json!(format!("#/components/schemas/{verify_name}")),
        );
    }
    schemas.insert("Puzzle".into(),json!({"oneOf":puzzle_variants,"discriminator":{"propertyName":"kind","mapping":puzzle_mapping},"description":"Kind determines the data and optional solution shape. Additional output fields may be added. Game constraints and cross-field dimensions are enforced by the engine."}));
    schemas.insert("VerifyRequest".into(),json!({"oneOf":verify_variants,"discriminator":{"propertyName":"kind","mapping":verify_mapping},"description":"Choose the answer shape using kind. Engine validation also checks board dimensions and game rules against the seed and difficulty."}));
    schemas.insert("Error".into(),object(json!({"error":object(json!({"code":{"type":"string"},"message":{"type":"string"}}), &["code","message"])}), &["error"]));
    schemas
}

fn schema_ref(name: &str) -> Value {
    json!({"$ref":format!("#/components/schemas/{name}")})
}

fn integer(min: u64, max: u64) -> Value {
    json!({"type":"integer","minimum":min,"maximum":max})
}

fn array(items: Value, min: usize, max: usize) -> Value {
    json!({"type":"array","items":items,"minItems":min,"maxItems":max})
}

fn grid(cell: Value, sizes: &[usize]) -> Value {
    let variants: Vec<_> = sizes
        .iter()
        .map(|size| array(array(cell.clone(), *size, *size), *size, *size))
        .collect();
    if variants.len() == 1 {
        variants.into_iter().next().unwrap()
    } else {
        json!({"oneOf":variants})
    }
}

fn object(properties: Value, required: &[&str]) -> Value {
    json!({"type":"object","required":required,"properties":properties})
}

fn described(mut schema: Value, description: &str) -> Value {
    schema["description"] = json!(description);
    schema
}

fn body(schema: Value) -> Value {
    json!({"required":true,"content":{"application/json":{"schema":schema}}})
}

fn compute(summary: &str, schema: Value) -> Value {
    let mut responses = Map::new();
    responses.insert(
        "200".into(),
        json!({"description":"Success","content":{"application/json":{"schema":schema}}}),
    );
    for status in ["400", "401", "413", "429", "500", "503", "504"] {
        responses.insert(
            status.into(),
            json!({"$ref":"#/components/responses/Error"}),
        );
    }
    json!({"summary":summary,"responses":responses})
}

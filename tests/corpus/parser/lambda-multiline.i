result =
    xs.fold initial, acc x ->
        cleaned = clean x
        acc.append cleaned

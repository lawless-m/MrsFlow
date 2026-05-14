let v = Record.AddField([a=1], "b", () => 99, true)[b] in
    if Value.Is(v, type function) then v() else v

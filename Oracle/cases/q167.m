Table.ReplaceValue(
    #table({"s"}, {{"foo bar"},{"baz foo"}}),
    "foo",
    "FOO",
    Replacer.ReplaceText,
    {"s"})

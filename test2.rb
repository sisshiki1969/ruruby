p "foobar".scan(/../)               # => ["fo", "ob", "ar"]
p "foobar".scan("o")                # => ["o", "o"]
p "foobarbazfoobarbaz".scan(/ba./)  # => ["bar", "baz", "bar", "baz"]

p "foobar".scan(/(.)/) # => [["f"], ["o"], ["o"], ["b"], ["a"], ["r"]]

p "foobarbazfoobarbaz".scan(/(ba)(.)()/) # => [["ba", "r"], ["ba", "z"], ["ba", "r"], ["ba", "z"]]
h1 = { "a" => 100, "b" => 200 }.compare_by_identity
h2 = { "b" => 246, "c" => 300 }.compare_by_identity
h3 = { "b" => 357, "d" => 400 }.compare_by_identity
p({"a"=>100, "b"=>200}.compare_by_identity == h1.merge)
r1 = {}.compare_by_identity
r1["a"] = 100
r1["b"] = 200
r1["b"] = 246
r1["c"] = 300
puts r1 == h1.merge(h2)
r1 = {}.compare_by_identity
r1["a"] = 100
r1["b"] = 200
r1["b"] = 246
r1["b"] = 357
r1["c"] = 300
r1["d"] = 400
puts r1 == h1.merge(h2, h3) 
p({"a"=>100, "b"=>200}.compare_by_identity == h1)
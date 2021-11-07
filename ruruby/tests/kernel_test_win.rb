ary = __FILE__.split("\\").reverse
assert "kernel_test_win.rb", ary[0]
assert "tests", ary[1]

ary = __dir__.split("/").reverse
assert "tests", ary[0]

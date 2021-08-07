def assert(expected, actual)
  if expected != actual
    puts "expected:#{expected}, actual:#{actual}"
  else
    puts "OK #{actual}"
  end
end

errors = [
  ["a", NameError],
  ["break", SyntaxError],
  ["Integer('z')", ArgumentError],
  ["5 * :sym", TypeError],
  ["4 / 0", ZeroDivisionError],
  ["500.chr", RangeError],
]
errors.each do | code, error|
  begin
    eval code
  rescue SyntaxError, StandardError => err
    assert error, err.class
  else
    raise
  end
end
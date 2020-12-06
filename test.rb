a = []
begin
  a << "begin"
  raise Exception.new
  a << "unreachable"
rescue StandardError
  a << "StandardError"
rescue
  a << "Exception"
end
assert ["begin", "Exception"], a
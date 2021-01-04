class MyException < StandardError
end

begin
  raise MyException
rescue => e
  puts e
  puts e.class
  puts e.message
end
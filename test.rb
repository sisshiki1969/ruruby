def fn 
  a = []
  begin
    a << "begin"
    return 100
    a << "unreachable"
  rescue StandardError => ex
    a << "StandardError"
  rescue Exception => ex
    a << "Exception"
  else
    a << "else"
  ensure
    a << "ensure"
  end
  a
end
    
puts fn
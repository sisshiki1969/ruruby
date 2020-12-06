  begin
    p "begin"
    raise Exception.new
    p "unreachable"
  rescue StandardError => ex
    p "StandardError #{ex.inspect}"
  rescue Exception => ex
    p "Exception #{ex.inspect}"
  else
    p "else"
  ensure
    p "ensure"
  end

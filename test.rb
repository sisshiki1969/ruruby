def ask_yes_no(question, default=nil)
  result = nil

  #while result.nil? do
    result = case ask "#{question} [#{default_answer}]"
             when /^y/i then true
             when /^n/i then false
             when /^$/  then default
             else            nil
             end
  #end

  return result
end
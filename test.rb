def iich
  yield(15)
end

sum = 15
iich{|x| sum = sum + x }
puts sum
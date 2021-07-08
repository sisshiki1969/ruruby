class A
  def each
    for i in 0..2
      yield i
    end
    #raise StopIteration
  end
end

a = A.new

for elem in a
  puts elem
end
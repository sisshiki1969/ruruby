def func2(&block)
  block.call
end

def func1(&block)
  func2(&block)
end

file = "A"
func1(){p file} #=> "A"
func2(){p file} #=> "A"
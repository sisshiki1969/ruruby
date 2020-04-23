str = "Yet Another Ruby Hacker"
enum = Enumerator.new(str, :scan, /\w+/)
upcase = Proc.new {|x| x.upcase}
p enum.map(&upcase)

ary = str.scan(/\w+/)
enum = Enumerator.new(ary)
p enum.map(&upcase)
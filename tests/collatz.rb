# This benchmarking program is based on the following blog.
# https://rheotommy.hatenablog.com/entry/2020/07/18/205343#Ruby
# by RheoTommy

def collatz(i)
  cnt = 0
  while i != 1 do
    cnt += 1
    if i % 2 == 0
      i /= 2
    else
      i *= 3
      i += 1
    end
  end
  cnt
end

n = 1000000 #gets.to_i
acc = 0
n.times do |i|
  acc += collatz(i + 1)
  acc %= 1000000007
end
puts(acc)
str = Array.new(10_000, 'abc').join(',')
1_000.times { str.scan('abc') }
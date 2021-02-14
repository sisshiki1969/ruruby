a = []
10.times { |i| a << i }
a.each { |e| 
    puts e
    if e == 3
        class Array
            def each
                3
            end
        end
    end
}
puts a.each
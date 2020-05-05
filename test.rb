def foo
    begin
        puts "begin 2"
        begin
            puts "begin 1"
            return 100
            puts "never"
        ensure
            puts "ensure 1"
        end
    ensure
        puts "ensure 2"
    end
end

puts foo    # => 100


block = Proc.new {|x| x.upcase }
p ["These", "are", "pencils"].map(&block)
s = "<do></do><txt>windows</txt>there is a pen."
s.gsub(/<(txt)>.*<\/(\1)>/, "_")
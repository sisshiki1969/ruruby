s = "ab" * 1000_000_000 + "ac"
s.gsub(/(a|b|ab)*bc/, "_")
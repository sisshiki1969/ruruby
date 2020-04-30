str = "<p><a href=\"http://example.com\">example.com</a></p>"
/<a href="(.*?)">(.*?)<\/a>/ === str
p($&)
p($')
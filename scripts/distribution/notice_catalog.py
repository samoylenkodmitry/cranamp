"""Make the packaged HTML notices readable in Cranamp's offline About screen."""

import json
from html.parser import HTMLParser
from pathlib import Path
import sys


class CatalogParser(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.notices = []
        self.current = None
        self.in_title = False
        self.libraries = []
        self.library = None
        self.in_library_name = False

    def handle_starttag(self, tag, attrs):
        if tag == "section":
            if self.current is not None:
                raise ValueError("Nested license section")
            self.current = {"title": "", "text": ""}
            self.libraries = []
        elif self.current is not None:
            if tag == "h2":
                self.in_title = True
            elif tag in ("pre", "p", "ul"):
                self.current["text"] += "\n\n"
            elif tag == "li":
                self.library = ""
            elif tag == "a":
                self.current["text"] += dict(attrs).get("href", "") + " "
                self.in_library_name = self.library == ""

    def handle_endtag(self, tag):
        if self.current is None:
            return
        if tag == "h2":
            self.in_title = False
        elif tag == "a":
            self.in_library_name = False
        elif tag == "li":
            self.current["text"] += "\n"
            if self.library:
                self.libraries.append(self.library.strip())
            self.library = None
        elif tag == "section":
            notice = {key: value.strip() for key, value in self.current.items()}
            if not all(notice.values()):
                raise ValueError("Empty license section")
            if self.libraries:
                notice["text"] = notice["title"] + "\n\n" + notice["text"]
                notice["title"] = self.libraries[0]
                if len(self.libraries) > 1:
                    count = len(self.libraries) - 1
                    notice["title"] += f" and {count} {'other' if count == 1 else 'others'}"
            self.notices.append(notice)
            self.current = None

    def handle_data(self, data):
        if self.current is not None:
            self.current["title" if self.in_title else "text"] += data
            if self.in_library_name:
                self.library += data


def catalog(html):
    parser = CatalogParser()
    parser.feed(html)
    parser.close()
    if parser.current is not None or not parser.notices:
        raise ValueError("Incomplete or empty license catalog")
    return sorted(parser.notices, key=lambda notice: notice["title"].casefold())


if __name__ == "__main__":
    source, destination = map(Path, sys.argv[1:])
    destination.write_text(json.dumps(catalog(source.read_text(encoding="utf-8")),
                                      ensure_ascii=False), encoding="utf-8")

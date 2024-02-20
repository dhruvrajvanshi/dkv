import json
import sys

def delete(dict, key):
    dict = dict.copy()
    if key in dict:
        del dict[key]
    return dict
tests = [delete(test, "skipped") for test in json.load(sys.stdin)
         if "tags" not in test or test["tags"] != "cluster"
         ]
json.dump(tests, sys.stdout, indent=2)


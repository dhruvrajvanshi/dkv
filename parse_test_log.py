import sys
import re
def main():
    lines = [line for line in sys.stdin.readlines() if line.startswith("Summary")]
    # example line:
    # Summary: version: 1.0.0, total tests: 50, passed: 5, rate: 10.00%
    # extract the version, total tests, passed, and rate

    regex = re.compile(r"Summary: version: (\d+\.\d+\.\d+), total tests: (?P<total>\d+), passed: (?P<passed>\d+), rate: (\d+\.\d+)%")
    total = 0
    passed = 0
    for line in lines:
        total += int(regex.match(line).group("total"))
        passed += int(regex.match(line).group("passed"))

    print(f"Total tests: {total}, Passed: {passed}, Rate: {passed/total*100:.2f}%")

if __name__ == '__main__':
    main()

import sys
import re
import json
def main():
    lines = sys.stdin.readlines()
    # example line:
    # Summary: version: 1.0.0, total tests: 50, passed: 5, rate: 10.00%
    # extract the version, total tests, passed, and rate

    regex = re.compile(r"Summary: version: (\d+\.\d+\.\d+), total tests: (?P<total>\d+), passed: (?P<passed>\d+), rate: (\d+\.\d+)%")
    total = 0
    passed = 0
    for line in lines:
        if not line.startswith("Summary"):
            continue
        total += int(regex.match(line).group("total"))
        passed += int(regex.match(line).group("passed"))

    print(f"Total tests: {total}, Passed: {passed}, Rate: {passed/total*100:.2f}%")
    with open("pages/test_summary.json", "w") as f:
        json.dump({
            "schemaVersion": 1,
            "label": "Redis compatibility",
            "message": f"{passed/total*100:.2f}% tests passed",
        }, f)

    test_suite = []
    with open("compatibility-test-suite-for-redis/cts.json") as f:
        test_suite = json.load(f)
    test_suite_by_name = {test["name"]: test for test in test_suite}

    with open("pages/test_summary.html", "w") as f:
        regex = re.compile(r"test: (?P<test>.+) (?P<result>(passed|failed|skipped))")
        rows = ""
        for line in lines:
            if not line.startswith("test: "):
                continue
            match = regex.match(line)
            if not match:
                print(f"Unexpected test line: {line}", file=sys.stderr)
                continue
            test = match.group("test").removesuffix(" tags")
            result = match.group("result")
            test_json = json.dumps(test_suite_by_name[test], indent=2)
            emoji = {
                "passed": "✅",
                "failed": "❌",
                "skipped": "⚠️",
            }[result]
            rows += f"""
                <tr>
                    <td>
                        <details style="margin-bottom: 0">
                            <summary>{test}</summary>
                            <pre><code class="language-json">{test_json}</code></pre>
                        </details>
                    </td>
                    <td>{emoji}&nbsp;{result.capitalize()}</td>
                </tr>
            """
        f.write(f"""
        <html>
        <head>
            <title>Redis compatibility test results</title>
            <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/@picocss/pico@2/css/pico.min.css" />
        </head>
        <body>
            <main class="container">
                <h1>Redis compatibility test results</h1>
                <h2>Summary</h2>
                <p>{passed}/{total} ({passed*100/total:.2f}%) tests passed</p>
                <progress value="{passed}" max="{total}">{passed}/{total}</progress>
                <table>
                    <tr>
                        <th>Test</th>
                        <th>Result</th>
                    </tr>
                    {rows}
                </table>
            </main>

            <link href="prism.css" rel="stylesheet" />
            <script src="prism.js"></script>
        </body>
        </html>
        """)

if __name__ == '__main__':
    main()

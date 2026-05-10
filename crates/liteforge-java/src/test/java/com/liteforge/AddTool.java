package com.liteforge;

/** Simple test tool that adds two numbers supplied as JSON. */
public class AddTool implements Tool {
    @Override
    public String name() {
        return "add";
    }

    @Override
    public String description() {
        return "Add two numbers";
    }

    @Override
    public String parametersSchemaJson() {
        return "{"
                + "\"type\":\"object\","
                + "\"properties\":{"
                + "\"a\":{\"type\":\"number\"},"
                + "\"b\":{\"type\":\"number\"}"
                + "},"
                + "\"required\":[\"a\",\"b\"]"
                + "}";
    }

    @Override
    public String execute(String argsJson) {
        // Minimal parser — we trust the tests to send well-formed input.
        double a = extract(argsJson, "a");
        double b = extract(argsJson, "b");
        return "{\"result\":" + (a + b) + "}";
    }

    private static double extract(String json, String key) {
        String needle = "\"" + key + "\":";
        int i = json.indexOf(needle);
        if (i < 0) {
            throw new IllegalArgumentException("missing key " + key);
        }
        int start = i + needle.length();
        int end = start;
        while (end < json.length()) {
            char c = json.charAt(end);
            if (c == ',' || c == '}' || c == ' ') break;
            end++;
        }
        return Double.parseDouble(json.substring(start, end).trim());
    }
}

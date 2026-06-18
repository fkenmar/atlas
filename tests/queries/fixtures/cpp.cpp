// Fixture for queries/cpp/tags.scm — one construct per extraction rule.
// Not compiled by cargo; it only needs to parse.

#include <string>
#include "config.h"

namespace service {

class Service {
public:
    Service(int count);
    int total() const;

protected:
    int adjust(int delta);

private:
    int helper(int x);
    int count_;
};

struct Point {
    int x;
    int y;
};

enum class Level {
    Low,
    High
};

using Id = int;

int add(int a, int b) {
    return add(a, b);
}

}  // namespace service

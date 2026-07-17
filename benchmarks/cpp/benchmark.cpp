#include <algorithm>
#include <cmath>
#include <iomanip>
#include <iostream>
#include <vector>

constexpr long long ROUNDS = 2;
constexpr long long INTEGER_ITERATIONS = 750'000;
constexpr long long FLOAT_ITERATIONS = 150'000;
constexpr long long LIST_SIZE = 100'000;

long long integer_work() {
    long long state = 1;
    long long checksum = 0;
    for (long long i = 0; i < INTEGER_ITERATIONS; ++i) {
        state = (state * 1'664'525 + 1'013'904'223 + i) % 2'147'483'647;
        checksum = (checksum + state) % 9'223'372'036'854'775'000LL;
    }
    return checksum;
}

double floating_work() {
    double checksum = 0.0;
    for (long long i = 0; i < FLOAT_ITERATIONS; ++i) {
        const double x = static_cast<double>(i + 1) * 0.00001;
        checksum += std::sin(x) * std::cos(x) + std::sqrt(x + 1.0) + std::log(x + 1.0);
    }
    return checksum;
}

long long list_work() {
    std::vector<long long> values;
    values.reserve(LIST_SIZE);
    long long state = 7;
    for (long long i = 0; i < LIST_SIZE; ++i) {
        state = (state * 48'271 + i) % 2'147'483'647;
        values.push_back(state);
    }
    std::sort(values.begin(), values.end());
    const auto middle = static_cast<std::size_t>(LIST_SIZE / 2);
    return values.front() + values[middle] + values.back() + static_cast<long long>(values.size());
}

int main() {
    long long integer_checksum = 0;
    double floating_checksum = 0.0;
    long long list_checksum = 0;

    for (long long round_index = 0; round_index < ROUNDS; ++round_index) {
        integer_checksum += integer_work() + round_index;
        floating_checksum += floating_work();
        list_checksum += list_work();
    }

    std::cout << "integer=" << integer_checksum << '\n';
    std::cout << "float=" << std::fixed << std::setprecision(6) << floating_checksum << '\n';
    std::cout << "list=" << list_checksum << '\n';
}

#define BOOST_TEST_MODULE FirstPassHandler Test
#include <boost/test/included/unit_test.hpp>

BOOST_AUTO_TEST_CASE(example_test_case)
{
  int i = 1;
  BOOST_TEST(i);
  BOOST_TEST(i == 2);
}

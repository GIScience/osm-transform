ors-preprocessor:
	g++-5 ors-preprocessor.cpp -o ors-preprocessor --std=c++11 -m64 -lpthread -lz -lexpat -lbz2 -lconfig++ -lboost_regex -lboost_system -O3
#-ltcmalloc

.PHONY:
clean:
	rm -f ors-preprocessor

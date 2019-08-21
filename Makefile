ors-preprocessor:
	g++ ors-preprocessor.cpp -o $@ --std=c++11 -lpthread -lz -lexpat -lbz2 -lconfig++ -lboost_regex -O3
#-ltcmalloc

.PHONY:
clean:
	rm -f ors-preprocessor

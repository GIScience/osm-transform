ors-preprocessor:
	g++ ors-preprocessor.cpp -o ors-preprocessor --std=c++11 -m64 -lpthread -lz -lexpat -lbz2 -lconfig++ -lgdal -lboost_regex -lboost_system -O3
#-ltcmalloc

srtm:
	g++ test/srtm.cpp -o srtm --std=c++11 -m64 -lpthread -lz -lexpat -lbz2 -lconfig++ -lgdal -lboost_system -lboost_filesystem -O3

gmted:
	g++ test/gmted.cpp -o gmted --std=c++11 -m64 -lpthread -lz -lexpat -lbz2 -lconfig++ -lgdal -lboost_system -lboost_filesystem -O3

.PHONY:
clean:
	@rm -f ors-preprocessor
	@rm -f srtm

FROM ubuntu:bionic as build

WORKDIR /app

RUN apt-get update
RUN apt-get install -y git automake autoconf make g++ libsdl1.2-dev libdbus-1-dev libncurses-dev
RUN git clone https://github.com/dosbox-staging/dosbox-staging.git --branch svn/RELEASE_0_74

WORKDIR /app/dosbox-staging

RUN rm ./src/debug/debug.cpp
COPY ./0.74/debug.cpp ./src/debug

RUN ./autogen.sh
ARG CPPFLAGS=-I/usr/include/dbus-1.0\ -I/usr/lib/x86_64-linux-gnu/dbus-1.0/include
ARG LIBS=-ldbus-1
RUN ./configure --enable-debug
RUN make

ENTRYPOINT ["top", "-b"]

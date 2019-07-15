#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <errno.h>
#include <sys/types.h>
#include <string.h>

#define MAX_CONNECTIONS 10
#define MAX_PACKET_SIZE 0x1000

typedef enum _packet_type {
  data_read = 0x0,
  data_write = 0x1,
  data_reset = 0x2,
} packet_type;

#pragma pack(1)
typedef struct _packet_data {
  packet_type type;
  uint64_t offset;
  uint64_t length;
  char data[0];
} packet_data;

static char LOG_OUTPUT = 0;

#define LOG(args...) do { if (LOG_OUTPUT) printf(args); } while(0);
#define ELOG(args...) do { if (LOG_OUTPUT) fprintf(stderr, args); } while(0);

int main(int argc, char **argv) {
  if (argc == 2) {
    LOG_OUTPUT = 1;
  }

  int sockfd = 0, connfd = 0;
  struct sockaddr_in address = { 0 };
  ssize_t bytes_read = 0;
  ssize_t saved_data_length = 0;
  char* saved_data = NULL;
  char* packet_buffer = malloc(MAX_PACKET_SIZE);
  int err = 0;
  packet_data* datagram = NULL;

  sockfd = socket(PF_INET, SOCK_STREAM, 0);
  if (sockfd < 0) {
    perror("socket");
    exit(EXIT_FAILURE);
  }
  if (setsockopt(sockfd, SOL_SOCKET, SO_REUSEADDR, &(int){ 1 }, sizeof(int)) < 0)
    perror("setsockopt(SO_REUSEADDR) failed");

  address.sin_family = AF_INET;
  address.sin_port = htons(8080);
  address.sin_addr.s_addr = INADDR_ANY;

  err = bind(sockfd, (struct sockaddr*)&address, sizeof(address));
  if (err < 0) {
      ELOG("bind returned error: %d\n", err);
      perror("bind");
      exit(EXIT_FAILURE);
  }

  listen(sockfd, MAX_CONNECTIONS);

  while(1) {
    connfd = accept(sockfd, (struct sockaddr*)NULL, NULL);

    LOG("got a new connection\n");

    bytes_read = read(connfd, packet_buffer, MAX_PACKET_SIZE);

    LOG("bytes_read: 0x%zX\n", bytes_read);

    if (bytes_read < sizeof(packet_data)) {
      ELOG("packet not large enough\n");
      goto cleanup;
    }

    datagram = (packet_data*)packet_buffer;

    switch (datagram->type) {
case data_read:
      LOG("got a data read packet\n");
      if (saved_data != NULL && datagram->offset + datagram->length <= saved_data_length) {
        write(connfd, packet_buffer + datagram->offset, datagram->length);
      }
      break;

case data_write:
      LOG("got a data write packet\n");
      // NOTE: Who cares about checking the offset? Nobody would ever provide bad data
      if (saved_data != NULL && datagram->length <= saved_data_length) {
        memcpy(saved_data + datagram->offset, datagram->data, datagram->length);
      }
      break;

case data_reset:
      LOG("got a data reset packet\n");
      if (datagram->length > bytes_read - sizeof(*datagram)) {
        ELOG("datagram length is invalid\n");
        goto cleanup;
      }

      if (saved_data != NULL) {
        free(saved_data);
      }

      saved_data = malloc(datagram->length);
      saved_data_length = datagram->length;

      LOG("0x%lX, 0x%lX\n", datagram->length, bytes_read - sizeof(*datagram));

      memcpy(saved_data, datagram->data, datagram->length);
      break;

default:
      ELOG("got an unknown datagram type: %d\n", datagram->type);
    }

cleanup:
    close(connfd);
  }
}

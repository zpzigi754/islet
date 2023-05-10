#include <gflags/gflags.h>

#include "support.h"
#include "certifier.h"
#include "simulated_enclave.h"
#include "cc_helpers.h"

#include <sys/socket.h>
#include <arpa/inet.h>
#include <netinet/in.h>
#include <netdb.h>
#include <openssl/ssl.h>
#include <openssl/rsa.h>
#include <openssl/x509.h>
#include <openssl/evp.h>
#include <openssl/rand.h>
#include <openssl/hmac.h>
#include <openssl/err.h>
#include <pthread.h>
#include <unistd.h>

// tensorflow
#include "tensorflow/lite/kernels/kernel_util.h"
#include "tensorflow/lite/kernels/register.h"
#include "tensorflow/lite/string_util.h"
#include <random>
#include <vector>
#include <iostream>
#include <stdlib.h>
#include <stdio.h>
#include <unistd.h>

// operations are: cold-init, get-certifier, run-runtime-server
DEFINE_bool(print_all, false,  "verbose");
DEFINE_string(operation, "", "operation");

DEFINE_string(policy_host, "localhost", "address for policy server");
DEFINE_int32(policy_port, 8123, "port for policy server");
DEFINE_string(data_dir, "./data/", "directory for application data");

DEFINE_string(runtime_host, "localhost", "address for runtime");
DEFINE_int32(runtime_model_port, 8124, "port for runtime (used to deliver model)");
DEFINE_int32(runtime_data_port, 8125, "port for runtime (used to deliver data for device1)");
DEFINE_int32(runtime_data_port2, 8126, "port for runtime (used to deliver data for device2)");

DEFINE_string(policy_store_file, "store.bin", "policy store file name");
DEFINE_string(platform_file_name, "platform_file.bin", "platform certificate");
DEFINE_string(platform_attest_endorsement, "platform_attest_endorsement.bin", "platform endorsement of attest key");
DEFINE_string(attest_key_file, "attest_key_file.bin", "attest key");
DEFINE_string(measurement_file, "example_app.measurement", "measurement");

// runtime performs four possible roles
//    cold-init: This creates application keys and initializes the policy store.
//    get-certifier: This obtains the app admission cert naming the public app key from the service.
//    run-runtime-ml-server: This runs the ML runtime as server
//    run-runtime-fl-server: This runs the FL runtime as server

#include "../certifier-data/policy_key.cc"
#include "../common/word_model.h"
#include "../common/util.h"

static cc_trust_data* app_trust_data = nullptr;
static char *ckpt_path = "./checkpoint/model.ckpt";
static char *aggr_ckpt_path = "./checkpoint/aggr_model.ckpt";
static char *local_ckpt_path[] = {"./checkpoint/local_model0.ckpt", "./checkpoint/local_model1.ckpt"};
static const int num_fl_devices = 2;
static int fl_model_idx = 0;
static int aggr_model_send = 0;
static pthread_mutex_t aggr_mutex;
static WordPredictionModel word_model;

using namespace std;

void send_model(secure_authenticated_channel& channel) {
  if (word_model.is_initialized() == false) {
    printf("model not initialized yet\n");
    return;
  }
  int n = channel.write(word_model.get_size(), word_model.get());
  printf("send model done: %d\n", n);
}

void send_trained_model(secure_authenticated_channel& channel, char *path) {
  unsigned char trained_weights[256 * 1024] = {0,};
  size_t len = read_file(path, trained_weights, sizeof(trained_weights));
  if (len == 0) {
    printf("ckpt read fail\n");
    return;
  }
  channel.write(len, trained_weights);
}

typedef struct _thread_arg{
  void (*callback)(secure_authenticated_channel& channel);
  int port;
} thread_arg;

void read_model_from_model_provider(secure_authenticated_channel& channel) {
  while (1) {
    string model;
    int n = channel.read(&model);
    if (n <= 0) {
      sleep(0);
      continue;
    }
    printf("read model. size: %d\n", n);
    word_model.init((unsigned char *)model.data(), n);

    // a simple response to let the peer know no problem here
    channel.write(sizeof(n), (unsigned char*)&n);
  }
}

// traditional ML: training at server
void read_data_from_devices(secure_authenticated_channel& channel) {
  while (1) {
    // 1. receive device data
    const char *request_str = "download_tflite_model";
    string out;
    int n = channel.read(&out);
    if (n <= 0) {
      sleep(0);
      continue;
    }
    if (strcmp(out.data(), request_str) == 0) {
      send_model(channel);
      continue;
    }

    // 2. do training
    pthread_mutex_lock(&aggr_mutex);
    printf("---- do training.... ----\n");
    word_model.train((char *)out.data(), ckpt_path);
    printf("---- training done! ----\n");

    // 3. send a new model
    printf("---- send trained weights ----\n");
    send_trained_model(channel, ckpt_path);
    pthread_mutex_unlock(&aggr_mutex);
  }
}

// federated learning: what server does is aggregation only
void read_local_model_from_devices(secure_authenticated_channel& channel) {
  while (1) {
    // 1. receive device data
    const char *request_str = "download_tflite_model";
    string out;
    int n = channel.read(&out);
    if (n <= 0) {
      sleep(0);
      continue;
    }
    if (strcmp(out.data(), request_str) == 0) {
      send_model(channel);
      continue;
    }

    // 2. gather local models
    save_as_file(local_ckpt_path[fl_model_idx], (unsigned char *)out.data(), n);
    fl_model_idx++;

    while (1) {
      pthread_mutex_lock(&aggr_mutex);
      if (fl_model_idx >= num_fl_devices) {
        printf("---- do aggregation to build a global model ---\n");
        int r = word_model.aggregate(local_ckpt_path, aggr_ckpt_path);
        if (r == 0) {
          fl_model_idx = 0;
          aggr_model_send = num_fl_devices;
          printf("---- aggregation done! ---\n");
        }
        else
          printf("aggregation fail\n");
        pthread_mutex_unlock(&aggr_mutex);
        break;
      } else {
        pthread_mutex_unlock(&aggr_mutex);
        if (aggr_model_send > 0)
          break;
        sleep(0);
      }
    }

    // 3. send the global model down to devices
    // By this point, we can make sure we have an aggregated model.
    send_trained_model(channel, aggr_ckpt_path);
    aggr_model_send -= 1;
  }
}

void *thread_func(void *arg) {
  thread_arg *targ = (thread_arg*)arg;

  printf("thread run: port number: %d\n", targ->port);
  server_dispatch(FLAGS_runtime_host, targ->port,
        app_trust_data->serialized_policy_cert_,
        app_trust_data->private_auth_key_,
        app_trust_data->private_auth_key_.certificate(),
        targ->callback);
}

void test_aggregation() {
  unsigned char model[256 * 1024] = {0,};
  size_t len = read_file("model.tflite", model, sizeof(model));
  if (len == 0) {
    return;
  }
  word_model.init(model, len);
  word_model.aggregate(local_ckpt_path, aggr_ckpt_path);
}

int main(int an, char** av) {
  gflags::ParseCommandLineFlags(&an, &av, true);
  an = 1;

  if (FLAGS_operation == "") {
    printf("runtime.exe --print_all=true|false --operation=op --policy_host=policy-host-address --policy_port=policy-host-port\n");
    printf("\t --data_dir=-directory-for-app-data --runtime_host=runtime-host-address --runtime_model_port=runtime-model-port --runtime_data_port=runtime-data-port\n");
    printf("\t --policy_cert_file=self-signed-policy-cert-file-name --policy_store_file=policy-store-file-name\n");
    printf("Operations are: cold-init, get-certifier, run-runtime-ml-server, run-runtime-fl-server\n");
    return 0;
  }

  SSL_library_init();
  string enclave_type("simulated-enclave");
  string purpose("authentication");

  string store_file(FLAGS_data_dir);
  store_file.append(FLAGS_policy_store_file);
  app_trust_data = new cc_trust_data(enclave_type, purpose, store_file);
  if (app_trust_data == nullptr) {
    printf("couldn't initialize trust object\n");
    return 1;
  }

  // Init policy key info
  if (!app_trust_data->init_policy_key(initialized_cert_size, initialized_cert)) {
    printf("Can't init policy key\n");
    return 1;
  }

  // Init simulated enclave
  string attest_key_file_name(FLAGS_data_dir);
  attest_key_file_name.append(FLAGS_attest_key_file);
  string platform_attest_file_name(FLAGS_data_dir);
  platform_attest_file_name.append(FLAGS_platform_attest_endorsement);
  string measurement_file_name(FLAGS_data_dir);
  measurement_file_name.append(FLAGS_measurement_file);
  string attest_endorsement_file_name(FLAGS_data_dir);
  attest_endorsement_file_name.append(FLAGS_platform_attest_endorsement);

  if (!app_trust_data->initialize_simulated_enclave_data(attest_key_file_name,
      measurement_file_name, attest_endorsement_file_name)) {
    printf("Can't init simulated enclave\n");
    return 1;
  }

  // Standard algorithms for the enclave
  string public_key_alg("rsa-2048");
  string symmetric_key_alg("aes-256");
  string hash_alg("sha-256");
  string hmac_alg("sha-256-hmac");

  // Carry out operation
  int ret = 0;
  if (FLAGS_operation == "cold-init") {
    if (!app_trust_data->cold_init(public_key_alg,
        symmetric_key_alg, hash_alg, hmac_alg)) {
      printf("cold-init failed\n");
      ret = 1;
    }
  } else if (FLAGS_operation == "get-certifier") {
    if (!app_trust_data->certify_me(FLAGS_policy_host, FLAGS_policy_port)) {
      printf("certification failed\n");
      ret = 1;
    }
  } else if ((FLAGS_operation == "run-runtime-ml-server") || (FLAGS_operation == "run-runtime-fl-server")) {
    if (!app_trust_data->warm_restart()) {
      printf("warm-restart failed\n");
      ret = 1;
      goto done;
    }
    printf("running as server\n");

    // create two threads, one for model and the other two for data
    pthread_t model_thread, data_thread, data_thread2;
    int model_status, data_status, data_status2;
    thread_arg model_arg = {
      .callback = read_model_from_model_provider,
      .port = FLAGS_runtime_model_port,
    };
    thread_arg data_arg = {
      .callback = read_data_from_devices,
      .port = FLAGS_runtime_data_port,
    };
    thread_arg data_arg2 = {
      .callback = read_data_from_devices,
      .port = FLAGS_runtime_data_port2,
    };

    if (FLAGS_operation == "run-runtime-fl-server") {
      data_arg.callback = read_local_model_from_devices;
      data_arg2.callback = read_local_model_from_devices;
    }

    pthread_mutex_init(&aggr_mutex, NULL);

    pthread_create(&model_thread, NULL, thread_func, (void*)&model_arg);
    pthread_create(&data_thread, NULL, thread_func, (void*)&data_arg);
    pthread_create(&data_thread2, NULL, thread_func, (void*)&data_arg2);

    // join
    pthread_join(model_thread, (void**)&model_status);
    pthread_join(data_thread, (void**)&data_status);
    pthread_join(data_thread2, (void**)&data_status2);

    pthread_mutex_destroy(&aggr_mutex);
  } else {
    printf("Unknown operation\n");
  }

done:
  // app_trust_data->print_trust_data();
  app_trust_data->clear_sensitive_data();
  if (app_trust_data != nullptr) {
    delete app_trust_data;
  }
  word_model.finalize();
  return ret;
}
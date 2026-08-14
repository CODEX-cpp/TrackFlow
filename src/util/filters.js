'use strict';

import Vue from 'vue';
import { seconds_to_duration, friendlydate } from './time';

import moment from 'moment';

Vue.filter('iso8601', function (timestamp) {
  return moment.parseZone(timestamp).format();
});

Vue.filter('friendlytime', function (timestamp) {
  return friendlydate(timestamp);
});

Vue.filter('friendlyduration', function (seconds) {
  return seconds_to_duration(seconds);
});
